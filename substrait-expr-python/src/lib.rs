use std::sync::Arc;

use pyo3::prelude::*;
use substrait::proto::Type;
use substrait_expr::builder;
use substrait_expr::builder::ExpressionsBuilder as InnerExpressionsBuilder;
use substrait_expr::helpers;
use substrait_expr::helpers::registry::ExtensionsRegistry;
use substrait_expr::helpers::types::TypeExt;

#[pyclass]
struct ExpressionsBuilder {
    inner: InnerExpressionsBuilder,
}

#[pyclass]
struct TypesOnlySchemaBuilder {
    inner: Arc<builder::schema::TypesOnlySchemaBuilder>,
    registry: Arc<helpers::registry::ExtensionsRegistry>,
}

impl TypeBuilderFactory for builder::schema::TypesOnlySchemaBuilder {
    fn make<'a>(&'a self) -> builder::schema::TypeBuilder<'a> {
        self.types()
    }
}

#[pymethods]
impl TypesOnlySchemaBuilder {
    #[new]
    pub fn new() -> Self {
        let registry = Arc::new(helpers::registry::ExtensionsRegistry::default());
        Self {
            inner: Arc::new(builder::schema::TypesOnlySchemaBuilder::new_with_types(
                registry.clone(),
            )),
            registry,
        }
    }

    pub fn types(&self) -> TypeBuilder {
        TypeBuilder {
            inner: self.inner.clone(),
            registry: self.registry.clone(),
        }
    }
}

#[pyclass]
struct SubstraitType {
    inner: Type,
    registry: Arc<ExtensionsRegistry>,
}

#[pymethods]
impl SubstraitType {
    fn __repr__(&self) -> String {
        self.inner.to_human_readable(&self.registry)
    }
}

trait TypeBuilderFactory {
    fn make<'a>(&'a self) -> builder::schema::TypeBuilder<'a>;
}

#[pyclass]
struct TypeBuilder {
    inner: Arc<dyn TypeBuilderFactory + Send + Sync>,
    registry: Arc<ExtensionsRegistry>,
}

#[pymethods]
impl TypeBuilder {
    pub fn int8(&self, nullable: Option<bool>) -> SubstraitType {
        SubstraitType {
            inner: helpers::types::i8(nullable.unwrap_or(true)),
            registry: self.registry.clone(),
        }
    }
}

#[pymodule]
fn _internal(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<SubstraitType>()?;
    m.add_class::<TypeBuilder>()?;
    m.add_class::<TypesOnlySchemaBuilder>()?;
    Ok(())
}
