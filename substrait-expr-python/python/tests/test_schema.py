import pytest

import substrait_expr

def test_types_builder():
    registry = substrait_expr.ExtensionsRegistry()

    assert repr(registry.types().i8()) == "i8?"
    assert repr(registry.types().i8(False)) == "i8"
    assert repr(registry.types().i8(True)) == "i8?"

    assert repr(registry.types().i16()) == "i16?"
    assert repr(registry.types().i16(False)) == "i16"
    assert repr(registry.types().i16(True)) == "i16?"

    assert repr(registry.types().i32()) == "i32?"
    assert repr(registry.types().i32(False)) == "i32"
    assert repr(registry.types().i32(True)) == "i32?"

    assert repr(registry.types().i64()) == "i64?"
    assert repr(registry.types().i64(False)) == "i64"
    assert repr(registry.types().i64(True)) == "i64?"

    assert repr(registry.types().fp32()) == "fp32?"
    assert repr(registry.types().fp32(False)) == "fp32"
    assert repr(registry.types().fp32(True)) == "fp32?"

    assert repr(registry.types().fp64()) == "fp64?"
    assert repr(registry.types().fp64(False)) == "fp64"
    assert repr(registry.types().fp64(True)) == "fp64?"

    assert repr(registry.types().string()) == "string?"
    assert repr(registry.types().string(False)) == "string"
    assert repr(registry.types().string(True)) == "string?"

    assert repr(registry.types().binary()) == "binary?"
    assert repr(registry.types().binary(False)) == "binary"
    assert repr(registry.types().binary(True)) == "binary?"

    assert repr(registry.types().timestamp()) == "timestamp?"
    assert repr(registry.types().timestamp(False)) == "timestamp"
    assert repr(registry.types().timestamp(True)) == "timestamp?"

    assert repr(registry.types().timestamp_tz()) == "timestamp_tz?"
    assert repr(registry.types().timestamp_tz(False)) == "timestamp_tz"
    assert repr(registry.types().timestamp_tz(True)) == "timestamp_tz?"

    assert repr(registry.types().date()) == "date?"
    assert repr(registry.types().date(False)) == "date"
    assert repr(registry.types().date(True)) == "date?"

    assert repr(registry.types().time()) == "time?"
    assert repr(registry.types().time(False)) == "time"
    assert repr(registry.types().time(True)) == "time?"

    assert repr(registry.types().interval_year()) == "interval_year?"
    assert repr(registry.types().interval_year(False)) == "interval_year"
    assert repr(registry.types().interval_year(True)) == "interval_year?"

    assert repr(registry.types().interval_day()) == "interval_day?"
    assert repr(registry.types().interval_day(False)) == "interval_day"
    assert repr(registry.types().interval_day(True)) == "interval_day?"

    assert repr(registry.types().uuid()) == "uuid?"
    assert repr(registry.types().uuid(False)) == "uuid"
    assert repr(registry.types().uuid(True)) == "uuid?"

    assert repr(registry.types().fixed_char(42)) == "fixedchar?<42>"
    assert repr(registry.types().fixed_char(42, False)) == "fixedchar<42>"
    assert repr(registry.types().fixed_char(42, True)) == "fixedchar?<42>"

    assert repr(registry.types().fixed_binary(42)) == "fixedbinary?<42>"
    assert repr(registry.types().fixed_binary(42, False)) == "fixedbinary<42>"
    assert repr(registry.types().fixed_binary(42, True)) == "fixedbinary?<42>"

    assert repr(registry.types().varchar(42)) == "varchar?<42>"
    assert repr(registry.types().varchar(42, False)) == "varchar<42>"
    assert repr(registry.types().varchar(42, True)) == "varchar?<42>"

    big_length = pow(2, 32) - 1
    with pytest.raises(ValueError, match=r".*greater than 2\^31-1"):
        repr(registry.types().fixed_char(big_length))
    with pytest.raises(ValueError, match=r".*greater than 2\^31-1"):
        repr(registry.types().fixed_binary(big_length))
    with pytest.raises(ValueError, match=r".*greater than 2\^31-1"):
        repr(registry.types().varchar(big_length))

    assert repr(registry.types().decimal(16, 8)) == "decimal?<16,8>"
    assert repr(registry.types().decimal(16, 8, False)) == "decimal<16,8>"
    assert repr(registry.types().decimal(16, 8, True)) == "decimal?<16,8>"

    with pytest.raises(ValueError, match=r"invalid precision \(40\)"):
        repr(registry.types().decimal(40, 8))
    with pytest.raises(ValueError, match=r"invalid scale \(12\) given precision \(10\)"):
        repr(registry.types().decimal(10, 12))

    assert repr(registry.types().list(registry.types().i32())) == "list?<i32?>"
    assert repr(registry.types().list(registry.types().i32(), False)) == "list<i32?>"
    assert repr(registry.types().list(registry.types().i32(), True)) == "list?<i32?>"

    assert repr(registry.types().map(registry.types().i32(), registry.types().string(False))) == "map?<i32?,string>"
    assert repr(registry.types().map(registry.types().i32(), registry.types().string(False), False)) == "map<i32?,string>"
    assert repr(registry.types().map(registry.types().i32(), registry.types().string(False), True)) == "map?<i32?,string>"

    assert repr(registry.types().struct_([registry.types().i32(), registry.types().string(False)])) == "struct?<i32?,string>"
    assert repr(registry.types().struct_([registry.types().i32(), registry.types().string(False)], False)) == "struct<i32?,string>"
    assert repr(registry.types().struct_([registry.types().i32(), registry.types().string(False)], True)) == "struct?<i32?,string>"

    def types_gen():
        yield registry.types().i32()
        yield registry.types().i64()

    assert repr(registry.types().struct_(types_gen())) == "struct?<i32?,i64?>"

def test_types_schema_builder():
    builder = substrait_expr.types_schema()
    builder.field(builder.types().i32()).field(builder.types().i64())