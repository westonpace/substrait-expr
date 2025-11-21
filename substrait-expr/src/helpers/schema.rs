use substrait::proto::{
    expression::{reference_segment::ReferenceType, ReferenceSegment},
    r#type::Struct,
    NamedStruct, Type,
};

use crate::{
    error::{Result, SubstraitExprError},
    util::HasRequiredPropertiesRef,
};

use super::{
    registry::ExtensionsRegistry,
    types::{self, nullability, TypeExt},
};

/// A schema that does not know types or names
///
/// This is also the only schema type that does not know
/// how many fields there are.
#[derive(Debug, Default, PartialEq)]
pub struct EmptySchema {
    registry: ExtensionsRegistry,
}

/// A field in a names-only schema
#[derive(PartialEq, Debug)]
pub struct NamesOnlySchemaNode {
    /// The name of this node
    ///
    /// This will be the empty string for the root node
    pub name: String,
    /// The names of this node's children
    pub children: Vec<NamesOnlySchemaNode>,
}

/// Represents a potentially nested schema where we only know the names and not
/// the types of the fields.
///
/// The root of the schema is `Vec<NamesOnlySchemaNode>` where each node represents
/// a single (possibly nested) column
#[derive(PartialEq, Debug)]
pub struct NamesOnlySchema {
    registry: ExtensionsRegistry,
    /// The root node of the schema
    pub root: NamesOnlySchemaNode,
}

impl NamesOnlySchema {
    /// Create a new names-only schema
    pub fn new(root_nodes: Vec<NamesOnlySchemaNode>) -> Self {
        Self {
            root: NamesOnlySchemaNode {
                name: String::new(),
                children: root_nodes,
            },
            registry: ExtensionsRegistry::default(),
        }
    }

    /// Create a names-only schema with the given registry
    pub fn new_with_registry(
        root_nodes: Vec<NamesOnlySchemaNode>,
        registry: ExtensionsRegistry,
    ) -> Self {
        Self {
            root: NamesOnlySchemaNode {
                name: String::new(),
                children: root_nodes,
            },
            registry,
        }
    }
}

impl NamesOnlySchemaNode {
    /// Determines the type of a scheam node
    ///
    /// Since we don't know types this will typically be unknown.  However,
    /// for nested fields, we know they must be of the Struct type.
    fn as_type(&self, unknown_type: &Type) -> Type {
        if self.children.is_empty() {
            unknown_type.clone()
        } else {
            types::struct_(
                true,
                self.children
                    .iter()
                    .map(|child| child.as_type(unknown_type))
                    .collect::<Vec<_>>(),
            )
        }
    }
}

struct NamesOnlySchemaNodeNamesDfsIter<'a> {
    stack: Vec<&'a NamesOnlySchemaNode>,
}

impl<'a> Iterator for NamesOnlySchemaNodeNamesDfsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.stack.pop();
        if let Some(next) = next {
            self.stack.extend(next.children.iter().rev());
            Some(&next.name)
        } else {
            None
        }
    }
}

/// A schema that knows the types (but not names) of its fields
#[derive(Debug, PartialEq)]
pub struct TypesOnlySchema {
    registry: ExtensionsRegistry,
    /// The root node of the schema
    pub root: Struct,
}

impl TypesOnlySchema {
    /// Create a new types-only schema
    pub fn new(root: Struct) -> Self {
        Self {
            root,
            registry: ExtensionsRegistry::default(),
        }
    }

    /// Create a types-only schema with a given registry
    pub fn new_with_registry(root: Struct, registry: ExtensionsRegistry) -> Self {
        Self { root, registry }
    }
}

/// A field in a schema that knows both types and names
#[derive(Debug, PartialEq)]
pub struct FullSchemaNode {
    /// The name of the field
    ///
    /// This will be the empty string for the root node
    pub name: String,
    /// The type of the field
    pub r#type: Type,
    /// The child types
    pub children: Vec<FullSchemaNode>,
}

/// A schema that knows both the types and names of its fields
#[derive(Debug, PartialEq)]
pub struct FullSchema {
    registry: ExtensionsRegistry,
    /// The root node of the schema
    pub root: FullSchemaNode,
}

impl FullSchema {
    /// Create a new full schema
    pub fn new(root: FullSchemaNode) -> Self {
        Self {
            root,
            registry: ExtensionsRegistry::default(),
        }
    }

    /// Create a full schema with the given registry
    pub fn new_with_registry(root: FullSchemaNode, registry: ExtensionsRegistry) -> Self {
        Self { root, registry }
    }
}

/// A schema represents what we know about the input to an expression
///
/// TODO: Expand, copy over content from crate docs
#[derive(PartialEq, Debug)]
pub enum SchemaInfo {
    Empty(EmptySchema),
    Names(NamesOnlySchema),
    Types(TypesOnlySchema),
    Full(FullSchema),
}

struct TypesOnlySchemaTypesDfsIter<'a> {
    stack: Vec<&'a Type>,
    include_inner: bool,
}

impl<'a> Iterator for TypesOnlySchemaTypesDfsIter<'a> {
    type Item = &'a Type;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.stack.pop();
            if let Some(next) = next {
                let children = next.children();
                self.stack.extend(children.iter().rev());
                if self.include_inner || children.is_empty() {
                    return Some(next);
                }
            } else {
                return None;
            }
        }
    }
}

struct FullSchemaFieldsDfsIter<'a> {
    stack: Vec<&'a FullSchemaNode>,
    include_inner: bool,
}

impl<'a> Iterator for FullSchemaFieldsDfsIter<'a> {
    type Item = &'a FullSchemaNode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.stack.pop();
            if let Some(next) = next {
                let children = &next.children;
                self.stack.extend(children.iter().rev());
                if self.include_inner || children.is_empty() {
                    return Some(next);
                }
            } else {
                return None;
            }
        }
    }
}

impl SchemaInfo {
    /// Return a reference to the schema's extensions registry
    ///
    /// This registry keeps track of the user defined types
    /// that are referenced by the schema
    pub fn extensions_registry(&self) -> &ExtensionsRegistry {
        match self {
            SchemaInfo::Empty(schm) => &schm.registry,
            SchemaInfo::Names(schm) => &schm.registry,
            SchemaInfo::Types(schm) => &schm.registry,
            SchemaInfo::Full(schm) => &schm.registry,
        }
    }

    /// Return true if this schema knows the names of its fields
    pub fn names_aware(&self) -> bool {
        match self {
            SchemaInfo::Empty(_) => false,
            SchemaInfo::Names(_) => true,
            SchemaInfo::Types(_) => false,
            SchemaInfo::Full(_) => true,
        }
    }

    /// Return true if this schema knows the types of its fields
    pub fn types_aware(&self) -> bool {
        match self {
            SchemaInfo::Empty(_) => false,
            SchemaInfo::Names(_) => false,
            SchemaInfo::Types(_) => true,
            SchemaInfo::Full(_) => true,
        }
    }

    /// Return true if this schema knows the number of fields
    pub fn len_aware(&self) -> bool {
        match self {
            SchemaInfo::Empty(_) => false,
            SchemaInfo::Names(_) => true,
            SchemaInfo::Types(_) => true,
            SchemaInfo::Full(_) => true,
        }
    }

    /// Returns an iterator through the names of the fields, in DFS order
    ///
    /// Returns an error if the schema does not know the names of its fields
    pub fn names_dfs<'a>(&'a self) -> Result<Box<dyn Iterator<Item = &'a str> + 'a>> {
        match self {
            SchemaInfo::Empty(_) => Err(SubstraitExprError::invalid_input(
                "Attempt to access field names when the schema is not name-aware",
            )),
            SchemaInfo::Names(names) => Ok(Box::new(NamesOnlySchemaNodeNamesDfsIter {
                stack: Vec::from_iter(names.root.children.iter()),
            })),
            SchemaInfo::Types(_) => Err(SubstraitExprError::invalid_input(
                "Attempt to access field names when the schema is not name-aware",
            )),
            SchemaInfo::Full(full) => Ok(Box::new(
                FullSchemaFieldsDfsIter {
                    stack: Vec::from_iter(full.root.children.iter().rev()),
                    include_inner: true,
                }
                .map(|node| node.name.as_str()),
            )),
        }
    }

    /// Returns an iterator through the types of the fields, in DFS order
    ///
    /// If the schema is empty this will return an empty iterator
    /// If this schema is names-only the types will all be the unknown type
    ///
    /// TODO: Explain include_inner, provide examples
    pub fn types_dfs<'a>(&'a self, include_inner: bool) -> Box<dyn Iterator<Item = Type> + 'a> {
        match self {
            SchemaInfo::Empty(_) => Box::new(std::iter::empty()),
            // TODO: SchemaInfo::Names is flat, SchemaInfo::Types is not.  Resolve this difference
            SchemaInfo::Names(names) => {
                let unknown_type = crate::builder::types::unknown(&names.registry);
                Box::new(
                    names
                        .root
                        .children
                        .iter()
                        .map(move |child| child.as_type(&unknown_type)),
                )
            }
            SchemaInfo::Types(type_info) => Box::new(
                TypesOnlySchemaTypesDfsIter {
                    stack: Vec::from_iter(type_info.root.types.iter().rev()),
                    include_inner,
                }
                .cloned(),
            ),
            SchemaInfo::Full(full) => Box::new(
                FullSchemaFieldsDfsIter {
                    stack: Vec::from_iter(full.root.children.iter().rev()),
                    include_inner,
                }
                .map(|node| node.r#type.clone()),
            ),
        }
    }

    /// Converts to a NamedStruct which is the closest equivalent SubstraitMessage
    pub fn to_substrait(self) -> NamedStruct {
        // TODO: Should include_inner be true here?
        let types = self.types_dfs(false).collect::<Vec<_>>();
        let names = if self.names_aware() {
            self.names_dfs()
                .unwrap()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        } else {
            types
                .iter()
                .enumerate()
                .map(|(idx, _)| format!("field_{}", idx))
                .collect::<Vec<_>>()
        };
        NamedStruct {
            names,
            r#struct: Some(Struct {
                nullability: nullability(false),
                types,
                ..Default::default()
            }),
        }
    }

    /// Return the type of the field referenced by `ref_seg`
    ///
    /// Returns an error if the reference does not refer to a field in the schema
    ///
    /// If types are not known then the returned type will be the unknown type
    pub fn resolve_type(&self, ref_seg: &ReferenceSegment) -> Result<Type> {
        match self {
            SchemaInfo::Empty(empty) => Ok(crate::builder::types::unknown(&empty.registry)),
            // TODO: Make sure a field exists before returning unknown
            SchemaInfo::Names(names) => Ok(crate::builder::types::unknown(&names.registry)),
            SchemaInfo::Types(type_info) => {
                let mut cur = &type_info.root.types;
                let mut _owned_cur = Vec::new();
                loop {
                    match ref_seg.reference_type.required("reference_type")? {
                        ReferenceType::StructField(struct_field) => {
                            let field = &cur[struct_field.field as usize];
                            if let Some(_child) = &struct_field.child {
                                let children = field.children();
                                if children.is_empty() {
                                    // TODO: fix error message to explain what happened
                                    return Err(SubstraitExprError::invalid_input(
                                        "Invalid reference",
                                    ));
                                }
                                _owned_cur = children.into_iter().cloned().collect::<Vec<_>>();
                                cur = &_owned_cur;
                            } else {
                                return Ok(field.clone());
                            }
                        }
                        ReferenceType::ListElement(_list_element) => todo!(),
                        ReferenceType::MapKey(_map_key) => todo!(),
                    }
                }
            }
            SchemaInfo::Full(full) => {
                let mut cur_seg = ref_seg;
                let mut cur_children = &full.root.children;
                loop {
                    match cur_seg.reference_type.required("reference_type")? {
                        ReferenceType::StructField(struct_field) => {
                            // TODO: Bounds checking?
                            let field = &cur_children[struct_field.field as usize];
                            if let Some(child) = &struct_field.child {
                                let children = &field.children;
                                if children.is_empty() {
                                    // TODO: fix error message to explain what happened
                                    return Err(SubstraitExprError::invalid_input(
                                        "Invalid reference",
                                    ));
                                }
                                cur_children = children;
                                cur_seg = child.as_ref();
                            } else {
                                return Ok(field.r#type.clone());
                            }
                        }
                        ReferenceType::ListElement(_list_element) => todo!(),
                        ReferenceType::MapKey(_map_key) => todo!(),
                    }
                }
            }
        }
    }
}
