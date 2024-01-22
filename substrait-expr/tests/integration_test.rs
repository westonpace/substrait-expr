use substrait_expr::builder::schema::SchemaBuildersExt;
use substrait_expr::helpers::schema::{EmptySchema, SchemaInfo};
use substrait_expr::helpers::types;
use substrait_expr::{
    builder::{BuilderParams, ExpressionsBuilder},
    functions::functions_arithmetic::FunctionsArithmeticExt,
    helpers::literals::literal,
};
use substrait_expr_macros::names_schema;

#[test]
pub fn test_schema_macros() {
    let schema = names_schema!({
        score: {},
        location: {
            x: {},
            y: {}
        }
    });
    let mut expected_builder = SchemaInfo::new_names();
    expected_builder
        .field("score")
        .nested("location", |builder| builder.field("x").field("y"));
    let expected = expected_builder.build();
    assert_eq!(schema, expected);
}

#[test]
pub fn test_ext_func() {
    let schema = SchemaInfo::Empty(EmptySchema::default());
    let builder = ExpressionsBuilder::new(schema, BuilderParams::new_loose());
    builder.functions().add(literal(3), literal(5));
}

#[test]
pub fn test_building_simple_expression() {
    let mut schema_builder = SchemaInfo::new_full();
    schema_builder
        .field("score", types::i32(false))
        .nested("location", false, |builder| {
            builder
                .field("x", types::fp32(false))
                .field("y", types::fp64(true))
        });
    let schema = schema_builder.build();

    let params = BuilderParams {
        allow_unknown_types: true,
        ..Default::default()
    };
    let builder = ExpressionsBuilder::new(schema, params);

    builder
        .add_expression(
            "sum",
            builder
                .functions()
                .add(
                    builder.fields().resolve_by_name("location.x").unwrap(),
                    literal(3.0_f32),
                )
                .build()
                .unwrap(),
        )
        .unwrap();

    let expressions = builder.build();
    dbg!(expressions);
}
