use std::sync::Arc;

use pyo3::prelude::*;
use substrait::proto::Type;
use substrait_expr::builder;
use substrait_expr::builder::ExpressionsBuilder as InnerExpressionsBuilder;
use substrait_expr::helpers;

#[pyclass]
struct ExpressionsBuilder {
    inner: InnerExpressionsBuilder,
}

#[pyclass]
struct TypesOnlySchemaBuilder {
    inner: Arc<builder::schema::TypesOnlySchemaBuilder>,
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
        Self {
            inner: Arc::new(builder::schema::TypesOnlySchemaBuilder::new()),
        }
    }

    pub fn types(&self) -> TypeBuilder {
        TypeBuilder {
            inner: self.inner.clone(),
        }
    }
}

#[pyclass]
struct SubstraitType {
    inner: Type,
}

trait TypeBuilderFactory {
    fn make<'a>(&'a self) -> builder::schema::TypeBuilder<'a>;
}

#[pyclass]
struct TypeBuilder {
    inner: Arc<dyn TypeBuilderFactory + Send + Sync>,
}

#[pymethods]
impl TypeBuilder {
    pub fn int8(&self, nullable: Option<bool>) -> SubstraitType {
        SubstraitType {
            inner: helpers::types::i8(nullable.unwrap_or(true)),
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
