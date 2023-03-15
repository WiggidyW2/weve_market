from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class AdjustedPriceRep(_message.Message):
    __slots__ = ["adjusted_price"]
    ADJUSTED_PRICE_FIELD_NUMBER: _ClassVar[int]
    adjusted_price: float
    def __init__(self, adjusted_price: _Optional[float] = ...) -> None: ...

class AdjustedPriceReq(_message.Message):
    __slots__ = ["type_id"]
    TYPE_ID_FIELD_NUMBER: _ClassVar[int]
    type_id: int
    def __init__(self, type_id: _Optional[int] = ...) -> None: ...

class MarketOrder(_message.Message):
    __slots__ = ["price", "quantity"]
    PRICE_FIELD_NUMBER: _ClassVar[int]
    QUANTITY_FIELD_NUMBER: _ClassVar[int]
    price: float
    quantity: int
    def __init__(self, quantity: _Optional[int] = ..., price: _Optional[float] = ...) -> None: ...

class MarketOrdersRep(_message.Message):
    __slots__ = ["market_orders"]
    MARKET_ORDERS_FIELD_NUMBER: _ClassVar[int]
    market_orders: _containers.RepeatedCompositeFieldContainer[MarketOrder]
    def __init__(self, market_orders: _Optional[_Iterable[_Union[MarketOrder, _Mapping]]] = ...) -> None: ...

class MarketOrdersReq(_message.Message):
    __slots__ = ["buy", "market", "type_id"]
    BUY_FIELD_NUMBER: _ClassVar[int]
    MARKET_FIELD_NUMBER: _ClassVar[int]
    TYPE_ID_FIELD_NUMBER: _ClassVar[int]
    buy: bool
    market: str
    type_id: int
    def __init__(self, type_id: _Optional[int] = ..., market: _Optional[str] = ..., buy: bool = ...) -> None: ...

class SystemIndexRep(_message.Message):
    __slots__ = ["copying", "invention", "manufacturing", "reactions", "research_me", "research_te"]
    COPYING_FIELD_NUMBER: _ClassVar[int]
    INVENTION_FIELD_NUMBER: _ClassVar[int]
    MANUFACTURING_FIELD_NUMBER: _ClassVar[int]
    REACTIONS_FIELD_NUMBER: _ClassVar[int]
    RESEARCH_ME_FIELD_NUMBER: _ClassVar[int]
    RESEARCH_TE_FIELD_NUMBER: _ClassVar[int]
    copying: float
    invention: float
    manufacturing: float
    reactions: float
    research_me: float
    research_te: float
    def __init__(self, manufacturing: _Optional[float] = ..., research_te: _Optional[float] = ..., research_me: _Optional[float] = ..., copying: _Optional[float] = ..., invention: _Optional[float] = ..., reactions: _Optional[float] = ...) -> None: ...

class SystemIndexReq(_message.Message):
    __slots__ = ["system_id"]
    SYSTEM_ID_FIELD_NUMBER: _ClassVar[int]
    system_id: int
    def __init__(self, system_id: _Optional[int] = ...) -> None: ...
