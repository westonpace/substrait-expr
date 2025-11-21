use std::{collections::BTreeMap, sync::RwLock};

use substrait::proto::extensions::{
    simple_extension_declaration::{ExtensionFunction, ExtensionType, MappingType},
    SimpleExtensionDeclaration, SimpleExtensionUri, SimpleExtensionUrn,
};

use crate::builder::functions::FunctionDefinition;

/// A qualified name has both a uri and a name
#[derive(PartialEq, Debug)]
pub struct QualifiedName {
    pub uri: String,
    pub name: String,
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.uri, self.name)
    }
}

#[derive(PartialEq, Clone, Debug)]

struct TypeRecord {
    uri: String,
    name: String,
    anchor: u32,
}

#[derive(PartialEq, Clone, Debug)]
struct FunctionRecord {
    uri: String,
    name: String,
    anchor: u32,
}

struct UriLookup {
    uris: BTreeMap<String, u32>,
    counter: u32,
}

impl UriLookup {
    pub fn new() -> Self {
        Self {
            uris: BTreeMap::new(),
            counter: 1,
        }
    }

    pub fn register(&mut self, uri: impl Into<String>) -> u32 {
        *self.uris.entry(uri.into()).or_insert_with(|| {
            let next = self.counter;
            self.counter += 1;
            next
        })
    }

    pub fn to_substrait(self) -> (Vec<SimpleExtensionUri>, Vec<SimpleExtensionUrn>) {
        let uris = self.uris
            .iter()
            .map(|entry| SimpleExtensionUri {
                extension_uri_anchor: *entry.1,
                uri: entry.0.clone(),
            })
            .collect::<Vec<_>>();
        let urns = self.uris
            .into_iter()
            .map(|entry| SimpleExtensionUrn {
                extension_urn_anchor: entry.1,
                urn: entry.0,
            })
            .collect::<Vec<_>>();
        (uris, urns)
    }
}

#[derive(PartialEq, Debug)]
struct RegistryInternal {
    functions: BTreeMap<String, FunctionRecord>,
    functions_inverse: BTreeMap<u32, FunctionRecord>,
    types: BTreeMap<String, TypeRecord>,
    types_inverse: BTreeMap<u32, TypeRecord>,
    counter: u32,
}

impl RegistryInternal {
    pub fn lookup_type(&self, anchor: u32) -> Option<QualifiedName> {
        self.types_inverse.get(&anchor).map(|record| QualifiedName {
            uri: record.uri.clone(),
            name: record.name.clone(),
        })
    }

    pub fn lookup_function(&self, anchor: u32) -> Option<QualifiedName> {
        self.functions_inverse
            .get(&anchor)
            .map(|record| QualifiedName {
                uri: record.uri.clone(),
                name: record.name.clone(),
            })
    }

    fn register_type(&mut self, uri: String, name: &str) -> u32 {
        let key = uri.clone() + name;
        let entry = self.types.entry(key);
        entry
            .or_insert_with(|| {
                let anchor = self.counter;
                self.counter += 1;
                let type_record = TypeRecord {
                    uri,
                    name: name.to_string(),
                    anchor,
                };
                self.types_inverse.insert(anchor, type_record.clone());
                type_record
            })
            .anchor
    }

    fn register_function(&mut self, uri: &str, name: &str) -> u32 {
        let key = uri.to_string() + name;
        let entry = self.functions.entry(key);
        entry
            .or_insert_with(|| {
                let anchor = self.counter;
                self.counter += 1;
                let function_record = FunctionRecord {
                    uri: uri.to_string(),
                    name: name.to_string(),
                    anchor,
                };
                self.functions_inverse
                    .insert(anchor, function_record.clone());
                function_record
            })
            .anchor
    }
}

/// Keeps track of extensions used within a plan
///
/// Substrait plans refer to extensions with "anchors".  These integer values are meant
/// to be a lightweight replacement for the uri/name pair.  This works well for serialization
/// but for an in-memory representation we want to be able to lookup these anchors quickly.
///
/// This type keeps track of maps between anchors and qualified names.
///
/// Note that the extensions registry is mutable.  Types and functions can be registered with
/// the extension registry.
///
/// The extensions registry is still Sync as it protects all access to itself with RwLock.
#[derive(Debug)]
pub struct ExtensionsRegistry {
    internal: RwLock<RegistryInternal>,
}

impl Default for ExtensionsRegistry {
    fn default() -> Self {
        Self {
            internal: RwLock::new(RegistryInternal {
                functions: BTreeMap::new(),
                types: BTreeMap::new(),
                functions_inverse: BTreeMap::new(),
                types_inverse: BTreeMap::new(),
                counter: 1,
            }),
        }
    }
}

impl PartialEq for ExtensionsRegistry {
    fn eq(&self, other: &Self) -> bool {
        *self.internal.read().unwrap() == *other.internal.read().unwrap()
    }
}

impl ExtensionsRegistry {
    /// Registers a new type with the extensions registry and returns an anchor to use
    ///
    /// If this is called multiple times with the same uri/name it will return the same anchor
    pub fn register_type(&self, uri: String, name: &str) -> u32 {
        let mut internal = self.internal.write().unwrap();
        internal.register_type(uri, name)
    }

    /// Registers a new function with the extensions registry and returns an anchor to use
    ///
    /// If this is called multiple times with the same uri/name it will return the same anchor
    pub fn register_function(&self, function: &FunctionDefinition) -> u32 {
        let mut internal = self.internal.write().unwrap();
        internal.register_function(&function.uri, &function.name)
    }

    /// Registers a new function with the extensions registry and returns an anchor to use
    ///
    /// If this is called multiple times with the same uri/name it will return the same anchor
    pub fn register_function_by_name(&self, uri: &str, name: &str) -> u32 {
        let mut internal = self.internal.write().unwrap();
        internal.register_function(uri, name)
    }

    /// Looks up the qualified name that corresponds to a type anchor
    pub fn lookup_type(&self, anchor: u32) -> Option<QualifiedName> {
        let internal = self.internal.read().unwrap();
        internal.lookup_type(anchor)
    }

    /// Looks up the qualified name that corresponds to a function anchor
    pub fn lookup_function(&self, anchor: u32) -> Option<QualifiedName> {
        let internal = self.internal.read().unwrap();
        internal.lookup_function(anchor)
    }

    fn add_types(
        &self,
        internal: &RegistryInternal,
        uris: &mut UriLookup,
        extensions: &mut Vec<SimpleExtensionDeclaration>,
    ) {
        for record in internal.types.values() {
            let uri_ref = uris.register(record.uri.clone());
            #[allow(deprecated)]
            let declaration = SimpleExtensionDeclaration {
                mapping_type: Some(MappingType::ExtensionType(ExtensionType {
                    extension_uri_reference: uri_ref,
                    extension_urn_reference: uri_ref,
                    type_anchor: record.anchor,
                    name: record.name.clone(),
                })),
            };
            extensions.push(declaration);
        }
    }

    fn add_functions(
        &self,
        internal: &RegistryInternal,
        uris: &mut UriLookup,
        extensions: &mut Vec<SimpleExtensionDeclaration>,
    ) {
        for record in internal.functions.values() {
            let uri_ref = uris.register(record.uri.clone());
            #[allow(deprecated)]
            let declaration = SimpleExtensionDeclaration {
                mapping_type: Some(MappingType::ExtensionFunction(ExtensionFunction {
                    extension_uri_reference: uri_ref,
                    extension_urn_reference: uri_ref,
                    function_anchor: record.anchor,
                    name: record.name.clone(),
                })),
            };
            extensions.push(declaration);
        }
    }

    /// Creates a substrait representation of the extensions registry
    ///
    /// This is typically placed in a top-level message such as ExtendedExpression or Plan
    pub fn to_substrait(&self) -> (Vec<SimpleExtensionUri>, Vec<SimpleExtensionUrn>, Vec<SimpleExtensionDeclaration>) {
        let mut uris = UriLookup::new();
        let mut extensions: Vec<SimpleExtensionDeclaration> = Vec::new();
        let internal = self.internal.read().unwrap();

        self.add_types(&internal, &mut uris, &mut extensions);
        self.add_functions(&internal, &mut uris, &mut extensions);

        let (uris, urns) = uris.to_substrait();

        (uris, urns, extensions)
    }
}
