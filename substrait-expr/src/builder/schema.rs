use std::iter::Peekable;
use std::str::Chars;
use std::sync::Arc;

use substrait::proto::expression::field_reference::{RootReference, RootType};
use substrait::proto::expression::reference_segment::{
    ListElement, MapKey, ReferenceType, StructField,
};
use substrait::proto::expression::{FieldReference, ReferenceSegment, RexType};
use substrait::proto::r#type::{Kind, Struct, UserDefined};
use substrait::proto::{Expression, Type};

use crate::error::{Result, SubstraitExprError};
use crate::helpers::expr::ExpressionExt;
use crate::helpers::literals::literal;
use crate::helpers::registry::ExtensionsRegistry;
use crate::helpers::schema::{
    FullSchema, FullSchemaNode, NamesOnlySchema, NamesOnlySchemaNode, SchemaInfo, TypesOnlySchema,
};
use crate::helpers::types::{nullability, NO_VARIATION, UNKNOWN_TYPE_NAME, UNKNOWN_TYPE_URI};

use super::functions::FunctionsBuilder;
use super::BuilderParams;

// ---------------- Builders for schemas --------------

/// A builder object for creating a particular user defined type
pub struct UserDefinedTypeBuilder {
    type_reference: u32,
}

impl UserDefinedTypeBuilder {
    /// Create an instance of the type with the given nullability
    ///
    /// Note, this does not consume the builder and can be called
    /// many times.
    pub fn with_nullability(&self, nullable: bool) -> Type {
        Type {
            kind: Some(Kind::UserDefined(UserDefined {
                nullability: nullability(nullable),
                type_parameters: vec![],
                type_reference: self.type_reference,
                type_variation_reference: NO_VARIATION,
            })),
        }
    }
}

/// A builder object for creating user defined types
pub struct TypeBuilder<'a> {
    registry: &'a ExtensionsRegistry,
}

impl<'a> TypeBuilder<'a> {
    /// Create an instance of the "unknown" type
    ///
    /// This is a special type when it comes to function resolution.  It will
    /// always match an argument, regardless of what type is expected.  However,
    /// if it is present, then the function's return type changes to unknown as\
    /// well.
    ///
    /// This type is normally used when the schema is unknown or not type aware.
    pub fn unknown(&self) -> Type {
        let type_reference = self
            .registry
            .register_type(UNKNOWN_TYPE_URI.to_string(), UNKNOWN_TYPE_NAME);
        Type {
            kind: Some(Kind::UserDefined(UserDefined {
                nullability: nullability(true),
                type_parameters: vec![],
                type_reference,
                type_variation_reference: NO_VARIATION,
            })),
        }
    }

    /// Create a builder that can create instances of a user defined type
    pub fn user_defined(
        &self,
        uri: impl Into<String>,
        name: impl AsRef<str>,
    ) -> UserDefinedTypeBuilder {
        let type_reference = self.registry.register_type(uri.into(), name.as_ref());
        UserDefinedTypeBuilder { type_reference }
    }
}

/// A builder for creating a types-only schema
pub struct TypesOnlySchemaBuilder {
    children: Vec<Type>,
    registry: Arc<ExtensionsRegistry>,
}

impl TypesOnlySchemaBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            registry: Arc::new(ExtensionsRegistry::default()),
        }
    }

    /// Create a new builder with the given type registry
    ///
    /// This is an advanced case and only needed if you are trying to maintain type
    /// anchors.
    pub fn new_with_types(registry: Arc<ExtensionsRegistry>) -> Self {
        Self {
            children: Vec::new(),
            registry,
        }
    }

    /// Add a new leaf field to the schema of the given type
    pub fn field(mut self, typ: Type) -> Self {
        self.children.push(typ);
        self
    }

    /// Add a new struct field to the schema
    pub fn nested(self, nullable: bool, build_func: impl FnOnce(Self) -> Self) -> Self {
        // TODO: Nested type registry needs to be incorporated into parent
        let nested_builder = build_func(Self::new());
        let types = nested_builder.build();
        if let SchemaInfo::Types(type_info) = types {
            let typ = Type {
                kind: Some(Kind::Struct(Struct {
                    types: type_info.root.types,
                    nullability: nullability(nullable),
                    ..Default::default()
                })),
            };
            self.field(typ)
        } else {
            panic!("Nested builder fn should return the provided builder")
        }
    }

    fn inner_build(self) -> (Struct, Arc<ExtensionsRegistry>) {
        (
            Struct {
                types: self.children,
                nullability: nullability(false),
                ..Default::default()
            },
            self.registry,
        )
    }

    /// Consume the builder to create a schema
    pub fn build(self) -> SchemaInfo {
        let (strct, registry) = self.inner_build();
        SchemaInfo::Types(TypesOnlySchema::new_with_registry(strct, registry))
    }

    /// Create a type builder to create user defined types
    pub fn types(&self) -> TypeBuilder {
        TypeBuilder {
            registry: &self.registry,
        }
    }
}

/// A builder object for a names-only schema
pub struct NamesOnlySchemaNodeBuilder {
    children: Vec<NamesOnlySchemaNode>,
    registry: Arc<ExtensionsRegistry>,
}

impl NamesOnlySchemaNodeBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            registry: Arc::new(ExtensionsRegistry::default()),
        }
    }

    /// Create a new builder with the given type registry
    ///
    /// This is an advanced case and only needed if you are trying to maintain type
    /// anchors.
    pub fn new_with_types(registry: Arc<ExtensionsRegistry>) -> Self {
        Self {
            children: Vec::new(),
            registry,
        }
    }

    /// Add a new leaf field to the schema with the given name
    pub fn field(mut self, name: impl Into<String>) -> Self {
        self.children.push(NamesOnlySchemaNode {
            name: name.into(),
            children: Vec::new(),
        });
        self
    }

    /// Add a new struct field to the schema with the given name
    pub fn nested(
        mut self,
        name: impl Into<String>,
        build_func: impl FnOnce(Self) -> Self,
    ) -> Self {
        let built = build_func(Self::new()).build();
        if let SchemaInfo::Names(built) = built {
            self.children.push(NamesOnlySchemaNode {
                name: name.into(),
                children: built.root.children,
            });
            self
        } else {
            panic!("Nested builder should return the result of builder.build()")
        }
    }

    /// Consume the builder to create a schema
    pub fn build(self) -> SchemaInfo {
        SchemaInfo::Names(NamesOnlySchema::new_with_registry(
            self.children,
            self.registry,
        ))
    }
}

/// A builder object for schemas that know both types and names
pub struct FullSchemaBuilder {
    nullable: bool,
    name: String,
    children: Vec<FullSchemaNode>,
    registry: Arc<ExtensionsRegistry>,
}

impl FullSchemaBuilder {
    /// Create a new builder
    fn new(nullable: bool, name: String) -> Self {
        Self {
            nullable,
            name,
            children: Vec::new(),
            registry: Arc::new(ExtensionsRegistry::default()),
        }
    }

    /// Add a leaf field with the given name and type
    pub fn field(mut self, name: impl Into<String>, typ: Type) -> Self {
        if let Some(Kind::Struct(_)) = typ.kind {
            panic!("FullSchemaBuilder::field was called with a struct.  Use FullSchemaBuilder::nested to create nested types");
        }
        self.children.push(FullSchemaNode {
            name: name.into(),
            r#type: typ,
            children: Vec::new(),
        });
        self
    }

    /// Add a struct field with the given name and children
    pub fn nested(
        mut self,
        name: impl Into<String>,
        nullable: bool,
        build_func: impl FnOnce(Self) -> Self,
    ) -> Self {
        // TODO: Merge type registry
        let (root, _) = build_func(Self::new(nullable, name.into())).inner_build();
        self.children.push(root);
        self
    }

    fn inner_build(self) -> (FullSchemaNode, Arc<ExtensionsRegistry>) {
        let typ = Type {
            kind: Some(Kind::Struct(Struct {
                nullability: nullability(self.nullable),
                types: self
                    .children
                    .iter()
                    .map(|child| &child.r#type)
                    .cloned()
                    .collect(),
                type_variation_reference: NO_VARIATION,
            })),
        };
        (
            FullSchemaNode {
                name: self.name,
                r#type: typ,
                children: self.children,
            },
            self.registry,
        )
    }

    /// Consume the builder to create a schema
    pub fn build(self) -> SchemaInfo {
        let (root, registry) = self.inner_build();
        SchemaInfo::Full(FullSchema::new_with_registry(root, registry))
    }
}

/// A trait that adds construction methods to SchemaInfo
pub trait SchemaBuildersExt {
    /// Create a builder that can be used to create a names-only schema
    fn new_names() -> NamesOnlySchemaNodeBuilder;
    /// Create a builder that can be used to create a types-only schema
    fn new_types() -> TypesOnlySchemaBuilder;
    /// Create a builder that can be used to create a full schema
    fn new_full() -> FullSchemaBuilder;
}

impl SchemaBuildersExt for SchemaInfo {
    fn new_names() -> NamesOnlySchemaNodeBuilder {
        NamesOnlySchemaNodeBuilder::new()
    }

    fn new_types() -> TypesOnlySchemaBuilder {
        TypesOnlySchemaBuilder::new()
    }

    fn new_full() -> FullSchemaBuilder {
        // The root node is always non-nullable (can't have an entire row be null) and has an empty
        // string as the name
        FullSchemaBuilder::new(false, String::new())
    }
}

/// A builder for expressions (field references) based on schemas
///
/// TODO: Add examples

pub trait ReferenceBuilder {
    /// References a field within the schema
    ///
    /// Can also be called again to reference a nested field with the chosen
    /// field
    fn field(&mut self, name: &str) -> Result<&mut dyn ReferenceBuilder>;
    /// Assuming the current node is a list column this will reference
    /// the idx'th item in the list.  If the list doesn't have enough
    /// items then NULL is returned (TODO: does substrait mandate this?)
    fn list_item(&mut self, idx: u32) -> Result<&mut dyn ReferenceBuilder>;
    /// Assuming the current node is a map column this will reference
    /// an item in the map with the given key.  If the map doesn't have
    /// an item matching this key then NULL is returned.
    ///
    /// `key` must be a literal
    fn map_item(&mut self, key: Expression) -> Result<&mut dyn ReferenceBuilder>;
    /// Consume the builder to create a reference
    fn build(&mut self) -> Result<Expression>;
}

struct AlwaysFaillingReferenceBuilder {
    reason: String,
}

impl ReferenceBuilder for AlwaysFaillingReferenceBuilder {
    fn field(&mut self, _: &str) -> Result<&mut dyn ReferenceBuilder> {
        Err(SubstraitExprError::InvalidInput(self.reason.clone()))
    }

    fn list_item(&mut self, _: u32) -> Result<&mut dyn ReferenceBuilder> {
        Err(SubstraitExprError::InvalidInput(self.reason.clone()))
    }

    fn map_item(&mut self, _: Expression) -> Result<&mut dyn ReferenceBuilder> {
        Err(SubstraitExprError::InvalidInput(self.reason.clone()))
    }

    fn build(&mut self) -> Result<Expression> {
        Err(SubstraitExprError::InvalidInput(self.reason.clone()))
    }
}

struct FullSchemaReferenceBuilder<'a> {
    cur_children: &'a Vec<FullSchemaNode>,
    parts: Vec<ReferenceSegment>,
    cur_path: String,
}

impl<'a> FullSchemaReferenceBuilder<'a> {
    fn new(schema: &'a FullSchema) -> Self {
        Self {
            cur_children: &schema.root.children,
            parts: Vec::new(),
            cur_path: String::new(),
        }
    }
}

// TODO: This is identical to the one used for the names schema.  Combine them somehow for DRY
impl<'a> ReferenceBuilder for FullSchemaReferenceBuilder<'a> {
    fn field(&mut self, name: &str) -> Result<&mut dyn ReferenceBuilder> {
        let name = name.to_string();
        let field_index = self
            .cur_children
            .iter()
            .position(|child| child.name == name);
        if let Some(field_index) = field_index {
            self.cur_path.push_str(&name);
            self.cur_children = &self.cur_children[field_index].children;
            self.parts.push(ReferenceSegment {
                reference_type: Some(ReferenceType::StructField(Box::new(StructField {
                    field: field_index as i32,
                    child: None,
                }))),
            });
            Ok(self)
        } else {
            Err(SubstraitExprError::InvalidInput(format!(
                "field {} does not exist at {} (no matching child)",
                name, self.cur_path
            )))
        }
    }

    fn list_item(&mut self, index: u32) -> Result<&mut dyn ReferenceBuilder> {
        self.parts.push(ReferenceSegment {
            reference_type: Some(ReferenceType::ListElement(Box::new(ListElement {
                offset: index as i32,
                child: None,
            }))),
        });
        Ok(self)
    }

    fn map_item(&mut self, key: Expression) -> Result<&mut dyn ReferenceBuilder> {
        self.parts.push(ReferenceSegment {
            reference_type: Some(ReferenceType::MapKey(Box::new(MapKey {
                map_key: Some(key.try_as_literal()?.clone()),
                child: None,
            }))),
        });
        Ok(self)
    }

    fn build(&mut self) -> Result<Expression> {
        let root_segment = self
            .parts
            .iter()
            .rev()
            .cloned()
            .reduce(|acc, mut el| {
                match el.reference_type.as_mut().unwrap() {
                    ReferenceType::StructField(struct_field) => {
                        struct_field.child = Some(Box::new(acc));
                    }
                    ReferenceType::ListElement(list_elem) => {
                        list_elem.child = Some(Box::new(acc));
                    }
                    ReferenceType::MapKey(map_key) => {
                        map_key.child = Some(Box::new(acc));
                    }
                };
                el
            })
            .ok_or_else(|| {
                SubstraitExprError::invalid_input("Attempt to create an empty field reference")
            })?;
        Ok(Expression {
            rex_type: Some(RexType::Selection(Box::new(FieldReference {
                reference_type: Some(
                    substrait::proto::expression::field_reference::ReferenceType::DirectReference(
                        root_segment,
                    ),
                ),
                root_type: Some(RootType::RootReference(RootReference {})),
            }))),
        })
    }
}

struct NamesOnlyReferenceBuilder<'a> {
    cur_children: &'a Vec<NamesOnlySchemaNode>,
    parts: Vec<ReferenceSegment>,
    cur_path: String,
}

impl<'a> NamesOnlyReferenceBuilder<'a> {
    fn new(schema: &'a NamesOnlySchema) -> Self {
        Self {
            cur_children: &schema.root.children,
            parts: Vec::new(),
            cur_path: String::new(),
        }
    }
}

impl<'a> ReferenceBuilder for NamesOnlyReferenceBuilder<'a> {
    fn field(&mut self, name: &str) -> Result<&mut dyn ReferenceBuilder> {
        let name = name.to_string();
        let field_index = self
            .cur_children
            .iter()
            .position(|child| child.name == name);
        if let Some(field_index) = field_index {
            self.cur_path.push_str(&name);
            self.cur_children = &self.cur_children[field_index].children;
            self.parts.push(ReferenceSegment {
                reference_type: Some(ReferenceType::StructField(Box::new(StructField {
                    field: field_index as i32,
                    child: None,
                }))),
            });
            Ok(self)
        } else {
            Err(SubstraitExprError::InvalidInput(format!(
                "field {} does not exist at {} (no matching child)",
                name, self.cur_path
            )))
        }
    }

    fn list_item(&mut self, index: u32) -> Result<&mut dyn ReferenceBuilder> {
        self.parts.push(ReferenceSegment {
            reference_type: Some(ReferenceType::ListElement(Box::new(ListElement {
                offset: index as i32,
                child: None,
            }))),
        });
        Ok(self)
    }

    fn map_item(&mut self, key: Expression) -> Result<&mut dyn ReferenceBuilder> {
        self.parts.push(ReferenceSegment {
            reference_type: Some(ReferenceType::MapKey(Box::new(MapKey {
                map_key: Some(key.try_as_literal()?.clone()),
                child: None,
            }))),
        });
        Ok(self)
    }

    fn build(&mut self) -> Result<Expression> {
        let root_segment = self
            .parts
            .iter()
            .rev()
            .cloned()
            .reduce(|acc, mut el| {
                match el.reference_type.as_mut().unwrap() {
                    ReferenceType::StructField(struct_field) => {
                        struct_field.child = Some(Box::new(acc));
                    }
                    ReferenceType::ListElement(list_elem) => {
                        list_elem.child = Some(Box::new(acc));
                    }
                    ReferenceType::MapKey(map_key) => {
                        map_key.child = Some(Box::new(acc));
                    }
                };
                el
            })
            .ok_or_else(|| {
                SubstraitExprError::invalid_input("Attempt to create an empty field reference")
            })?;
        Ok(Expression {
            rex_type: Some(RexType::Selection(Box::new(FieldReference {
                reference_type: Some(
                    substrait::proto::expression::field_reference::ReferenceType::DirectReference(
                        root_segment,
                    ),
                ),
                root_type: Some(RootType::RootReference(RootReference {})),
            }))),
        })
    }
}

enum NamedRefElement {
    Name(String),
    ListIndex(u32),
    MapLookup(String),
}

struct NamedRefIter<'a> {
    val: &'a str,
    chars: Peekable<Chars<'a>>,
    exhausted: bool,
    in_brackets: bool,
}

impl<'a> NamedRefIter<'a> {
    fn new(val: &'a str) -> Self {
        let chars = val.chars().peekable();
        Self {
            val,
            chars,
            exhausted: false,
            in_brackets: false,
        }
    }

    fn invalid(&self) -> SubstraitExprError {
        SubstraitExprError::InvalidInput(format!("Invalid field reference: {}", self.val))
    }
}

impl<'a> Iterator for NamedRefIter<'a> {
    type Item = Result<NamedRefElement>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        let mut part = String::new();
        if self.in_brackets {
            while let Some(chr) = self.chars.next() {
                if chr == ']' {
                    if part.is_empty() {
                        // x[] <-- empty brackets
                        return Some(Err(self.invalid()));
                    }
                    // E.g. if x[3].y then consume both ] and .
                    if let Some(chr) = self.chars.peek() {
                        if *chr == '.' {
                            self.chars.next();
                        } else {
                            // x[3]y is invalid
                            return Some(Err(self.invalid()));
                        }
                    }
                    self.in_brackets = false;
                    return if let Ok(idx) = part.parse::<u32>() {
                        Some(Ok(NamedRefElement::ListIndex(idx)))
                    } else {
                        Some(Ok(NamedRefElement::MapLookup(part)))
                    };
                } else {
                    part.push(chr);
                }
            }
            // foo[12 <-- No closing ]
            return Some(Err(self.invalid()));
        } else {
            while let Some(chr) = self.chars.next() {
                if chr == '.' {
                    return if part.is_empty() {
                        // . or x.. <-- empty segment
                        Some(Err(self.invalid()))
                    } else {
                        Some(Ok(NamedRefElement::Name(part)))
                    };
                } else if chr == '[' {
                    self.in_brackets = true;
                    return if part.is_empty() {
                        self.next()
                    } else {
                        Some(Ok(NamedRefElement::Name(part)))
                    };
                } else if chr == ']' {
                    // x] <-- missing [
                    return Some(Err(self.invalid()));
                } else {
                    part.push(chr);
                }
            }
        }
        self.exhausted = true;
        if part.is_empty() {
            None
        } else {
            Some(Ok(NamedRefElement::Name(part)))
        }
    }
}

/// Creates field reference expressions that reference a field in a schema
pub struct RefBuilder<'a> {
    schema: &'a SchemaInfo,
    params: &'a BuilderParams,
    functions: FunctionsBuilder<'a>,
}

impl<'a> RefBuilder<'a> {
    pub(crate) fn new(
        schema: &'a SchemaInfo,
        params: &'a BuilderParams,
        functions: FunctionsBuilder<'a>,
    ) -> Self {
        Self {
            schema,
            params,
            functions,
        }
    }

    /// Create a field reference from a "path string"
    ///
    /// TODO: Explain syntax
    /// TODO: Provide examples
    pub fn resolve_by_name(&self, name: &str) -> Result<Expression> {
        match &self.schema {
            SchemaInfo::Empty(_) => {
                if self.params.allow_late_name_lookup {
                    Ok(self.functions.lookup_field_by_name(name))
                } else {
                    Err(SubstraitExprError::InvalidInput(format!("Cannot lookup a field named {} because the input schema does not know the names", name)))
                }
            }
            SchemaInfo::Names(_) => {
                if !self.params.allow_unknown_types {
                    SubstraitExprError::invalid_input("Cannot reference fields when unknown types are disallowed and the schema is not type-aware");
                }
                let mut builder = self.field_builder();
                for path_part in NamedRefIter::new(name) {
                    match path_part? {
                        NamedRefElement::Name(name) => {
                            builder.field(&name)?;
                        }
                        NamedRefElement::ListIndex(idx) => {
                            builder.list_item(idx)?;
                        }
                        NamedRefElement::MapLookup(map_key) => {
                            builder.map_item(literal(map_key))?;
                        }
                    }
                }
                builder.build()
            }
            SchemaInfo::Types(_) => {
                if self.params.allow_late_name_lookup {
                    // Create a zero-arg UDF for late-bound function resolution
                    Ok(self.functions.lookup_field_by_name(name))
                } else {
                    Err(SubstraitExprError::InvalidInput(format!("Cannot lookup a field named {} because the input schema does not know the names", name)))
                }
            }
            SchemaInfo::Full(_) => {
                // TODO: This is identical to the path for names, fix for DRY
                let mut builder = self.field_builder();
                for path_part in NamedRefIter::new(name) {
                    match path_part? {
                        NamedRefElement::Name(name) => {
                            builder.field(&name)?;
                        }
                        NamedRefElement::ListIndex(idx) => {
                            builder.list_item(idx)?;
                        }
                        NamedRefElement::MapLookup(map_key) => {
                            builder.map_item(literal(map_key))?;
                        }
                    }
                }
                builder.build()
            }
        }
    }

    /// Create a builder that can be used to programmatically create a field reference
    pub fn field_builder(&self) -> Box<dyn ReferenceBuilder + 'a> {
        match &self.schema {
            SchemaInfo::Empty(_) => todo!(),
            SchemaInfo::Full(full) => Box::new(FullSchemaReferenceBuilder::new(full)),
            SchemaInfo::Names(names) => {
                if self.params.allow_unknown_types {
                    Box::new(NamesOnlyReferenceBuilder::new(names))
                } else {
                    Box::new(AlwaysFaillingReferenceBuilder { reason: "Cannot create field references when unknown types are disallowed and the schema is not type-aware".to_string() })
                }
            }
            SchemaInfo::Types(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as substrait_expr, helpers::types};
    use substrait_expr_macros::names_schema;

    use super::*;

    #[test]
    fn test_literals() {
        let lit = literal(1_i8);
        dbg!(lit);
    }

    #[test]
    fn test_names_only_schema_builder() {
        let expected = names_schema!({
            score: {},
            location: {
                x: {},
                y: {}
            }
        });
        let built = SchemaInfo::new_names()
            .field("score")
            .nested("location", |builder| builder.field("x").field("y"))
            .build();
        assert_eq!(expected, built);
    }

    #[test]
    fn test_resolve_by_name() {
        let schema = names_schema!({
            a: {
                b: {},
                c: {},
            },
            d: {}
        });
        let params = BuilderParams {
            allow_unknown_types: true,
            ..Default::default()
        };
        let functions = FunctionsBuilder::new(&schema);
        let ref_builder = RefBuilder {
            schema: &schema,
            params: &params,
            functions: functions,
        };

        let by_name = ref_builder.resolve_by_name("a.c[3]").unwrap();
        let by_builder = ref_builder
            .field_builder()
            .field("a")
            .unwrap()
            .field("c")
            .unwrap()
            .list_item(3)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(by_name, by_builder);

        let by_name = ref_builder.resolve_by_name("a[hello].b").unwrap();
        let by_builder = ref_builder
            .field_builder()
            .field("a")
            .unwrap()
            .map_item(literal("hello"))
            .unwrap()
            .field("b")
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(by_name, by_builder);
    }

    #[test]
    fn test_types_builder() {
        let schema = SchemaInfo::new_types()
            .field(types::i32(false))
            .nested(false, |builder| {
                builder.field(types::fp32(false)).field(types::fp64(true))
            })
            .build();

        assert!(schema.names_dfs().is_err());
        assert!(!schema.names_aware());
        assert!(schema.types_aware());

        let types = schema.types_dfs(true).collect::<Vec<_>>();
        let expected = vec![
            types::i32(false),
            types::struct_(false, vec![types::fp32(false), types::fp64(true)]),
            types::fp32(false),
            types::fp64(true),
        ];
        assert_eq!(expected, types);

        let types = schema.types_dfs(false).collect::<Vec<_>>();
        let expected = vec![types::i32(false), types::fp32(false), types::fp64(true)];
        assert_eq!(expected, types);
    }

    #[test]
    fn test_full_builder() {
        let schema = SchemaInfo::new_full()
            .field("score", types::i32(false))
            .nested("location", false, |builder| {
                builder
                    .field("x", types::fp32(false))
                    .field("y", types::fp64(true))
            })
            .build();

        assert_eq!(
            schema.names_dfs().unwrap().collect::<Vec<_>>(),
            vec![
                "score".to_string(),
                "location".to_string(),
                "x".to_string(),
                "y".to_string()
            ]
        );
        assert!(schema.names_aware());
        assert!(schema.types_aware());

        let types = schema.types_dfs(false).collect::<Vec<_>>();
        let expected = vec![types::i32(false), types::fp32(false), types::fp64(true)];
        assert_eq!(expected, types);
    }
}
