class SubstraitType(object):
    pass

class TypeBuilder(object):
    def int8(self, nullable: bool = ...) -> SubstraitType: ...

class TypesOnlySchemaBuilder(object):
    def __init__(self) -> None: ...
    def types(self) -> TypeBuilder: ...

