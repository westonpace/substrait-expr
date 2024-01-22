use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use substrait::proto::Type;
use substrait_expr::builder;
use substrait_expr::builder::ExpressionsBuilder as InnerExpressionsBuilder;
use substrait_expr::helpers;
use substrait_expr::helpers::registry::ExtensionsRegistry as InnerExtensionsRegistry;
use substrait_expr::helpers::types::TypeExt;

trait PythonErrorExt<T> {
    fn value_error(self) -> PyResult<T>;
}

impl<T> PythonErrorExt<T> for std::result::Result<T, substrait_expr::error::SubstraitExprError> {
    fn value_error(self) -> PyResult<T> {
        self.map_err(|err| PyValueError::new_err(err.to_string()))
    }
}

#[pyclass]
struct ExtensionsRegistry {
    inner: Arc<InnerExtensionsRegistry>,
}

#[pymethods]
impl ExtensionsRegistry {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(InnerExtensionsRegistry::default()),
        }
    }

    pub fn types(&self) -> TypeBuilder {
        TypeBuilder {
            registry: self.inner.clone(),
        }
    }
}

#[pyclass]
struct ExpressionsBuilder {
    inner: InnerExpressionsBuilder,
}

#[pyclass]
struct TypesOnlySchemaBuilder {
    inner: builder::schema::TypesOnlySchemaBuilder,
    registry: Arc<InnerExtensionsRegistry>,
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
        let registry = Arc::new(InnerExtensionsRegistry::default());
        Self {
            inner: builder::schema::TypesOnlySchemaBuilder::new_with_types(registry.clone()),
            registry,
        }
    }

    pub fn types(&self) -> TypeBuilder {
        TypeBuilder {
            registry: self.registry.clone(),
        }
    }

    pub fn field<'a>(mut slf: PyRefMut<'a, Self>, typ: &SubstraitType) -> PyRefMut<'a, Self> {
        slf.inner.field(typ.inner.clone());
        slf
    }
}

#[pyclass]
#[derive(Clone)]
struct SubstraitType {
    inner: Type,
    registry: Arc<InnerExtensionsRegistry>,
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
    registry: Arc<InnerExtensionsRegistry>,
}

macro_rules! simple_type {
    ($name:ident, $nullable: expr, $self: expr) => {
        SubstraitType {
            inner: helpers::types::$name($nullable.unwrap_or(true)),
            registry: $self.registry.clone(),
        }
    };
}

#[pymethods]
impl TypeBuilder {
    pub fn bool(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(bool, nullable, self)
    }
    pub fn i8(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(i8, nullable, self)
    }
    pub fn i16(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(i16, nullable, self)
    }
    pub fn i32(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(i32, nullable, self)
    }
    pub fn i64(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(i64, nullable, self)
    }
    pub fn fp32(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(fp32, nullable, self)
    }
    pub fn fp64(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(fp64, nullable, self)
    }
    pub fn string(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(string, nullable, self)
    }
    pub fn binary(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(binary, nullable, self)
    }
    pub fn timestamp(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(timestamp, nullable, self)
    }
    pub fn timestamp_tz(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(timestamp_tz, nullable, self)
    }
    pub fn date(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(date, nullable, self)
    }
    pub fn time(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(time, nullable, self)
    }
    pub fn interval_year(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(interval_year, nullable, self)
    }
    pub fn interval_day(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(interval_day, nullable, self)
    }
    pub fn uuid(&self, nullable: Option<bool>) -> SubstraitType {
        simple_type!(uuid, nullable, self)
    }
    pub fn fixed_char(&self, length: u32, nullable: Option<bool>) -> PyResult<SubstraitType> {
        Ok(SubstraitType {
            inner: helpers::types::fixed_char(length, nullable.unwrap_or(true)).value_error()?,
            registry: self.registry.clone(),
        })
    }
    pub fn varchar(&self, length: u32, nullable: Option<bool>) -> PyResult<SubstraitType> {
        Ok(SubstraitType {
            inner: helpers::types::varchar(length, nullable.unwrap_or(true)).value_error()?,
            registry: self.registry.clone(),
        })
    }
    pub fn fixed_binary(&self, length: u32, nullable: Option<bool>) -> PyResult<SubstraitType> {
        Ok(SubstraitType {
            inner: helpers::types::fixed_binary(length, nullable.unwrap_or(true)).value_error()?,
            registry: self.registry.clone(),
        })
    }
    pub fn decimal(
        &self,
        precision: u8,
        scale: u8,
        nullable: Option<bool>,
    ) -> PyResult<SubstraitType> {
        Ok(SubstraitType {
            inner: helpers::types::decimal(precision, scale, nullable.unwrap_or(true))
                .value_error()?,
            registry: self.registry.clone(),
        })
    }
    pub fn list(&self, item_type: &SubstraitType, nullable: Option<bool>) -> SubstraitType {
        SubstraitType {
            inner: helpers::types::list(item_type.inner.clone(), nullable.unwrap_or(true)),
            registry: self.registry.clone(),
        }
    }
    pub fn map(
        &self,
        key_type: &SubstraitType,
        value_type: &SubstraitType,
        nullable: Option<bool>,
    ) -> SubstraitType {
        SubstraitType {
            inner: helpers::types::map(
                key_type.inner.clone(),
                value_type.inner.clone(),
                nullable.unwrap_or(true),
            ),
            registry: self.registry.clone(),
        }
    }
    pub fn struct_(&self, types: &PyAny, nullable: Option<bool>) -> PyResult<SubstraitType> {
        let types = types
            .iter()?
            .map(|typ| Ok(typ?.extract::<SubstraitType>()?.inner.clone()))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(SubstraitType {
            inner: helpers::types::struct_(types, nullable.unwrap_or(true)),
            registry: self.registry.clone(),
        })
    }
}

#[pymodule]
fn _internal(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<SubstraitType>()?;
    m.add_class::<TypeBuilder>()?;
    m.add_class::<TypesOnlySchemaBuilder>()?;
    m.add_class::<ExtensionsRegistry>()?;
    Ok(())
}
