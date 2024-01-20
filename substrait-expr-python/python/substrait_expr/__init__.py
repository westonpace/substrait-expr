from . import _internal

def types_schema() -> _internal.TypesOnlySchemaBuilder:
    return _internal.TypesOnlySchemaBuilder()
