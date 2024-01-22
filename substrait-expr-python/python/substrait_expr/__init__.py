from . import _internal
from ._internal import ExtensionsRegistry

def types_schema() -> _internal.TypesOnlySchemaBuilder:
    return _internal.TypesOnlySchemaBuilder()
