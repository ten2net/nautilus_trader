# -------------------------------------------------------------------------------------------------
# <copyright file="ema_cross.py" company="Nautech Systems Pty Ltd">
#  Copyright (C) 2015-2019 Nautech Systems Pty Ltd. All rights reserved.
#  The use of this source code is governed by the license as found in the LICENSE.md file.
#  https://nautechsystems.io
# </copyright>
# -------------------------------------------------------------------------------------------------

from datetime import timedelta

from nautilus_trader.core.message import Event
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import TimeInForce
from nautilus_trader.model.objects import Price, Tick, BarSpecification, BarType, Bar, Instrument
from nautilus_trader.model.identifiers import Symbol, Label
from nautilus_trader.data.analyzers import SpreadAnalyzer, LiquidityAnalyzer
from nautilus_trader.trade.strategy import TradingStrategy
from nautilus_trader.trade.sizing import FixedRiskSizer

from nautilus_indicators.average.ema import ExponentialMovingAverage
from nautilus_indicators.atr import AverageTrueRange


class EMACrossPy(TradingStrategy):
    """"
    A simple moving average cross example strategy. When the fast EMA crosses
    the slow EMA then a STOP_MARKET atomic order is placed for that direction
    with a trailing stop and profit target at 1R risk.
    """

    def __init__(self,
                 symbol: Symbol,
                 bar_spec: BarSpecification,
                 risk_bp: float=100.0,
                 fast_ema: int=10,
                 slow_ema: int=20,
                 atr_period: int=20,
                 sl_atr_multiple: float=2.0):
        """
        Initializes a new instance of the EMACrossPy class.

        :param symbol: The symbol for the strategy.
        :param bar_spec: The bar specification for the strategy.
        :param risk_bp: The risk per trade (basis points).
        :param fast_ema: The fast EMA period.
        :param slow_ema: The slow EMA period.
        :param atr_period: The ATR period.
        :param sl_atr_multiple: The ATR multiple for stop-loss prices.
        """
        # Order id tag must be unique at trader level
        super().__init__(order_id_tag=symbol.code, bar_capacity=40)

        # Custom strategy variables
        self.symbol = symbol
        self.bar_type = BarType(symbol, bar_spec)
        self.risk_bp = risk_bp
        self.entry_buffer = 0  # instrument.tick_size
        self.SL_buffer = 0  # instrument.tick_size * 10
        self.SL_atr_multiple = sl_atr_multiple

        self.instrument = None
        self.position_sizer = None
        self.spread_analyzer = None
        self.liquidity = LiquidityAnalyzer()

        # Create the indicators for the strategy
        self.fast_ema = ExponentialMovingAverage(fast_ema)
        self.slow_ema = ExponentialMovingAverage(slow_ema)
        self.atr = AverageTrueRange(atr_period)

        # Register the indicators for updating
        self.register_indicator_bars(self.bar_type, self.fast_ema, self.fast_ema.update)
        self.register_indicator_bars(self.bar_type, self.slow_ema, self.slow_ema.update)
        self.register_indicator_bars(self.bar_type, self.atr, self.atr.update)

    def on_start(self):
        """
        This method is called when self.start() is called, and after internal start logic.
        """
        # Put custom code to be run on strategy start here (or pass)
        self.account_inquiry()
        self.instrument = self.get_instrument(self.symbol)
        self.entry_buffer = self.instrument.tick_size
        self.SL_buffer = self.instrument.tick_size * 10
        self.position_sizer = FixedRiskSizer(self.instrument)
        self.spread_analyzer = SpreadAnalyzer(self.instrument.tick_precision)

        self.request_bars(self.bar_type)
        self.subscribe_instrument(self.symbol)
        self.subscribe_bars(self.bar_type)
        self.subscribe_ticks(self.symbol)

    def on_tick(self, tick: Tick):
        """
        This method is called whenever a Tick is received by the strategy, and
        after the Tick has been processed by the base class.
        The received Tick object is then passed into this method.

        :param tick: The received tick.
        """
        # self.log.info(f"Received Tick({tick})")  # For demonstration purposes
        self.spread_analyzer.update(tick)

    def on_bar(self, bar_type: BarType, bar: Bar):
        """
        This method is called whenever the strategy receives a Bar, and after the
        Bar has been processed by the base class.
        The received BarType and Bar objects are then passed into this method.

        :param bar_type: The received bar type.
        :param bar: The received bar.
        """
        self.log.info(f"Received {bar_type} Bar({bar})")  # For demonstration purposes

        if not self.indicators_initialized():
            return  # Wait for indicators to warm up...

        if not self.has_ticks(self.symbol):
            return  # Wait for ticks...

        self.spread_analyzer.calculate_metrics()
        self.liquidity.update(self.spread_analyzer.average_spread, self.atr.value)

        # if self.liquidity.is_liquid
        if self.entry_orders_count() == 0 and self.is_flat():
            atomic_order = None

            # BUY LOGIC
            if self.fast_ema.value >= self.slow_ema.value:
                price_entry = Price(bar.high + self.entry_buffer + self.spread_analyzer.average_spread)
                price_stop_loss = Price(bar.low - (self.atr.value * self.SL_atr_multiple))
                price_take_profit = Price(price_entry + (price_entry - price_stop_loss))

                exchange_rate = self.get_exchange_rate(self.instrument.quote_currency)
                position_size = self.position_sizer.calculate(
                    equity=self.account.free_equity,
                    exchange_rate=exchange_rate,
                    risk_bp=self.risk_bp,
                    price_entry=price_entry,
                    price_stop_loss=price_stop_loss,
                    commission_rate_bp=0.15,
                    hard_limit=20000000,
                    units=1,
                    unit_batch_size=10000)
                if position_size.value > 0:
                    atomic_order = self.order_factory.atomic_stop_market(
                        symbol=self.symbol,
                        order_side=OrderSide.BUY,
                        quantity=position_size,
                        price_entry=price_entry,
                        price_stop_loss=price_stop_loss,
                        price_take_profit=price_take_profit,
                        label=Label('S1'),
                        time_in_force=TimeInForce.GTD,
                        expire_time=self.time_now() + timedelta(minutes=1))
                else:
                    self.log.info("Insufficient equity for BUY signal.")

            # SELL LOGIC
            elif self.fast_ema.value < self.slow_ema.value:
                price_entry = Price(bar.low - self.entry_buffer)
                price_stop_loss = Price(bar.high + (self.atr.value * self.SL_atr_multiple) + self.spread_analyzer.average_spread)
                price_take_profit = Price(price_entry - (price_stop_loss - price_entry))

                exchange_rate = self.get_exchange_rate(self.instrument.quote_currency)
                position_size = self.position_sizer.calculate(
                    equity=self.account.free_equity,
                    exchange_rate=exchange_rate,
                    risk_bp=self.risk_bp,
                    price_entry=price_entry,
                    price_stop_loss=price_stop_loss,
                    commission_rate_bp=0.15,
                    hard_limit=20000000,
                    units=1,
                    unit_batch_size=10000)

                if position_size.value > 0:  # Sufficient equity for a position
                    atomic_order = self.order_factory.atomic_stop_market(
                        symbol=self.symbol,
                        order_side=OrderSide.SELL,
                        quantity=position_size,
                        price_entry=price_entry,
                        price_stop_loss=price_stop_loss,
                        price_take_profit=price_take_profit,
                        label=Label('S1'),
                        time_in_force=TimeInForce.GTD,
                        expire_time=self.time_now() + timedelta(minutes=1))
                else:
                    self.log.info("Insufficient equity for SELL signal.")

            # ENTRY ORDER SUBMISSION
            if atomic_order is not None:
                self.submit_atomic_order(atomic_order, self.position_id_generator.generate())

        # TRAILING STOP LOGIC
        for trailing_stop in self.stop_loss_orders().values():
            if trailing_stop.is_working:
                # SELL SIDE ORDERS
                if trailing_stop.is_sell:
                    temp_price = Price(bar.low - (self.atr.value * self.SL_atr_multiple))
                    if temp_price > trailing_stop.price:
                        self.modify_order(trailing_stop, temp_price)
                # BUY SIDE ORDERS
                elif trailing_stop.is_buy:
                    temp_price = Price(
                        bar.high + (self.atr.value * self.SL_atr_multiple) + self.spread_analyzer.average_spread)
                    if temp_price < trailing_stop.price:
                        self.modify_order(trailing_stop, temp_price)

    def on_instrument(self, instrument: Instrument):
        """
        This method is called whenever the strategy receives an Instrument update.

        :param instrument: The received instrument.
        """
        if self.instrument.symbol.equals(instrument.symbol):
            self.instrument = instrument

        self.log.info(f"Updated instrument {instrument}.")

    def on_event(self, event: Event):
        """
        This method is called whenever the strategy receives an Event object,
        and after the event has been processed by the TradingStrategy base class.
        These events could be AccountEvent, OrderEvent, PositionEvent, TimeEvent.

        :param event: The received event.
        """
        # Put custom code for event handling here (or pass)
        pass

    def on_stop(self):
        """
        This method is called when self.stop() is called and after internal
        stopping logic.
        """
        # Put custom code to be run on strategy stop here (or pass)
        pass

    def on_reset(self):
        """
        This method is called when self.reset() is called, and after internal
        reset logic such as clearing the internally held bars, ticks and resetting
        all indicators.
        """
        # Put custom code to be run on a strategy reset here (or pass)
        self.spread_analyzer.reset()
        self.liquidity.reset()

    def on_dispose(self):
        """
        This method is called when self.dispose() is called. Dispose of any
        resources that had been used by the strategy here.
        """
        # Put custom code to be run on a strategy disposal here (or pass)
        self.unsubscribe_instrument(self.symbol)
        self.unsubscribe_bars(self.bar_type)
        self.unsubscribe_ticks(self.symbol)
