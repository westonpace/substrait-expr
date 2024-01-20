import substrait_expr

def test_types_schema_builder():
    builder = substrait_expr.types_schema()
    i8 = builder.types().int8()
    print(i8)
