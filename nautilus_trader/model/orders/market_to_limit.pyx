# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2022 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

from cpython.datetime cimport datetime
from libc.stdint cimport int64_t

from nautilus_trader.core.correctness cimport Condition
from nautilus_trader.core.datetime cimport dt_to_unix_nanos
from nautilus_trader.core.datetime cimport format_iso8601
from nautilus_trader.core.datetime cimport maybe_unix_nanos_to_dt
from nautilus_trader.core.uuid cimport UUID4
from nautilus_trader.model.c_enums.contingency_type cimport ContingencyType
from nautilus_trader.model.c_enums.contingency_type cimport ContingencyTypeParser
from nautilus_trader.model.c_enums.order_side cimport OrderSide
from nautilus_trader.model.c_enums.order_side cimport OrderSideParser
from nautilus_trader.model.c_enums.order_type cimport OrderType
from nautilus_trader.model.c_enums.order_type cimport OrderTypeParser
from nautilus_trader.model.c_enums.time_in_force cimport TimeInForce
from nautilus_trader.model.c_enums.time_in_force cimport TimeInForceParser
from nautilus_trader.model.events.order cimport OrderInitialized
from nautilus_trader.model.events.order cimport OrderUpdated
from nautilus_trader.model.identifiers cimport ClientOrderId
from nautilus_trader.model.identifiers cimport InstrumentId
from nautilus_trader.model.identifiers cimport OrderListId
from nautilus_trader.model.identifiers cimport StrategyId
from nautilus_trader.model.identifiers cimport TraderId
from nautilus_trader.model.objects cimport Quantity
from nautilus_trader.model.orders.base cimport Order


cdef class MarketToLimitOrder(Order):
    """
    Represents a `Market-To-Limit` order.

    Parameters
    ----------
    trader_id : TraderId
        The trader ID associated with the order.
    strategy_id : StrategyId
        The strategy ID associated with the order.
    instrument_id : InstrumentId
        The order instrument ID.
    client_order_id : ClientOrderId
        The client order ID.
    order_side : OrderSide {``BUY``, ``SELL``}
        The order side.
    quantity : Quantity
        The order quantity (> 0).
    time_in_force : TimeInForce {``GTC``, ``IOC``, ``FOK``, ``GTD``, ``DAY``}
        The order time in force.
    expire_time : datetime, optional
        The order expiration.
    init_id : UUID4
        The order initialization event ID.
    ts_init : int64
        The UNIX timestamp (nanoseconds) when the object was initialized.
    reduce_only : bool, default False
        If the order carries the 'reduce-only' execution instruction.
    display_qty : Quantity, optional
        The quantity of the limit order to display on the public book (iceberg).
    order_list_id : OrderListId, optional
        The order list ID associated with the order.
    contingency_type : ContingencyType, default ``NONE``
        The order contingency type.
    linked_order_ids : list[ClientOrderId], optional
        The order linked client order ID(s).
    parent_order_id : ClientOrderId, optional
        The order parent client order ID.
    tags : str, optional
        The custom user tags for the order. These are optional and can
        contain any arbitrary delimiter if required.

    Raises
    ------
    ValueError
        If `quantity` is not positive (> 0).
    ValueError
        If `time_in_force` is ``ON_OPEN`` or ``ON_CLOSE``.
    """

    def __init__(
        self,
        TraderId trader_id not None,
        StrategyId strategy_id not None,
        InstrumentId instrument_id not None,
        ClientOrderId client_order_id not None,
        OrderSide order_side,
        Quantity quantity not None,
        TimeInForce time_in_force,
        datetime expire_time,  # Can be None
        UUID4 init_id not None,
        int64_t ts_init,
        bint reduce_only=False,
        Quantity display_qty=None,
        OrderListId order_list_id=None,
        ContingencyType contingency_type=ContingencyType.NONE,
        list linked_order_ids=None,
        ClientOrderId parent_order_id=None,
        str tags=None,
    ):
        Condition.not_equal(time_in_force, TimeInForce.ON_OPEN, "time_in_force", "ON_OPEN`")
        Condition.not_equal(time_in_force, TimeInForce.ON_CLOSE, "time_in_force", "ON_CLOSE`")

        cdef int64_t expire_time_ns = 0
        if time_in_force == TimeInForce.GTD:
            # Must have an expire time
            Condition.not_none(expire_time, "expire_time")
            expire_time_ns = dt_to_unix_nanos(expire_time)
            Condition.true(expire_time_ns > 0, "`expire_time` cannot be <= UNIX epoch.")
        else:
            # Should not have an expire time
            Condition.none(expire_time, "expire_time")

        # Set options
        cdef dict options = {
            "display_qty": str(display_qty) if display_qty is not None else None,
            "expire_time_ns": expire_time_ns if expire_time_ns > 0 else None,
        }

        # Create initialization event
        cdef OrderInitialized init = OrderInitialized(
            trader_id=trader_id,
            strategy_id=strategy_id,
            instrument_id=instrument_id,
            client_order_id=client_order_id,
            order_side=order_side,
            order_type=OrderType.MARKET_TO_LIMIT,
            quantity=quantity,
            time_in_force=time_in_force,
            post_only=False,
            reduce_only=reduce_only,
            options=options,
            order_list_id=order_list_id,
            contingency_type=contingency_type,
            linked_order_ids=linked_order_ids,
            parent_order_id=parent_order_id,
            tags=tags,
            event_id=init_id,
            ts_init=ts_init,
        )
        super().__init__(init=init)

        self.price = None
        self.expire_time = expire_time
        self.expire_time_ns = expire_time_ns
        self.display_qty = display_qty

    cdef bint has_price_c(self) except *:
        return self.price is not None

    cdef bint has_trigger_price_c(self) except *:
        return False

    cpdef str info(self):
        """
        Return a summary description of the order.

        Returns
        -------
        str

        """
        cdef str expiration_str = "" if self.expire_time is None else f" {format_iso8601(self.expire_time)}"
        return (
            f"{OrderSideParser.to_str(self.side)} {self.quantity.to_str()} {self.instrument_id} "
            f"{OrderTypeParser.to_str(self.type)} @ {self.price} "
            f"{TimeInForceParser.to_str(self.time_in_force)}{expiration_str}"
        )

    cpdef dict to_dict(self):
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, object]

        """
        return {
            "trader_id": self.trader_id.value,
            "strategy_id": self.strategy_id.value,
            "instrument_id": self.instrument_id.value,
            "client_order_id": self.client_order_id.value,
            "venue_order_id": self.venue_order_id.value if self.venue_order_id else None,
            "position_id": self.position_id.value if self.position_id else None,
            "account_id": self.account_id.value if self.account_id else None,
            "last_trade_id": self.last_trade_id.value if self.last_trade_id else None,
            "type": OrderTypeParser.to_str(self.type),
            "side": OrderSideParser.to_str(self.side),
            "quantity": str(self.quantity),
            "price": str(self.price),
            "time_in_force": TimeInForceParser.to_str(self.time_in_force),
            "expire_time_ns": self.expire_time_ns if self.expire_time_ns > 0 else None,
            "reduce_only": self.is_reduce_only,
            "display_qty": str(self.display_qty) if self.display_qty is not None else None,
            "filled_qty": str(self.filled_qty),
            "avg_px": str(self.avg_px) if self.avg_px else None,
            "slippage": str(self.slippage),
            "status": self._fsm.state_string_c(),
            "order_list_id": self.order_list_id,
            "contingency_type": ContingencyTypeParser.to_str(self.contingency_type),
            "linked_order_ids": ",".join([o.value for o in self.linked_order_ids]) if self.linked_order_ids is not None else None,  # noqa
            "parent_order_id": self.parent_order_id,
            "tags": self.tags,
            "ts_last": self.ts_last,
            "ts_init": self.ts_init,
        }

    @staticmethod
    cdef MarketToLimitOrder create(OrderInitialized init):
        """
        Return a `Market-To-Limit` order from the given initialized event.

        Parameters
        ----------
        init : OrderInitialized
            The event to initialize with.

        Returns
        -------
        MarketToLimitOrder

        Raises
        ------
        ValueError
            If `init.type` is not equal to ``MARKET_TO_LIMIT``.

        """
        Condition.not_none(init, "init")
        Condition.equal(init.type, OrderType.MARKET_TO_LIMIT, "init.type", "OrderType")

        cdef str display_qty_str = init.options.get("display_qty")

        return MarketToLimitOrder(
            trader_id=init.trader_id,
            strategy_id=init.strategy_id,
            instrument_id=init.instrument_id,
            client_order_id=init.client_order_id,
            order_side=init.side,
            quantity=init.quantity,
            time_in_force=init.time_in_force,
            expire_time=maybe_unix_nanos_to_dt(init.options.get("expire_time_ns")),
            reduce_only=init.reduce_only,
            display_qty=Quantity.from_str_c(display_qty_str) if display_qty_str is not None else None,
            init_id=init.id,
            ts_init=init.ts_init,
            order_list_id=init.order_list_id,
            contingency_type=init.contingency_type,
            linked_order_ids=init.linked_order_ids,
            parent_order_id=init.parent_order_id,
            tags=init.tags,
        )

    cdef void _updated(self, OrderUpdated event) except *:
        if self.venue_order_id != event.venue_order_id:
            self._venue_order_ids.append(self.venue_order_id)
            self.venue_order_id = event.venue_order_id
        if event.quantity is not None:
            self.quantity = event.quantity
            self.leaves_qty = Quantity(self.quantity - self.filled_qty, self.quantity.precision)
        if event.price is not None:
            self.price = event.price

    cdef void _set_slippage(self) except *:
        if self.side == OrderSide.BUY:
            self.slippage = self.avg_px - self.price
        elif self.side == OrderSide.SELL:
            self.slippage = self.price - self.avg_px
