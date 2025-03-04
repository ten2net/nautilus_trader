// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2024 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use std::{error::Error, fs::File, io::BufReader, path::Path};

use csv::{Reader, ReaderBuilder};
use flate2::read::GzDecoder;
use nautilus_core::nanos::UnixNanos;
use nautilus_model::{
    data::{
        delta::OrderBookDelta,
        depth::{OrderBookDepth10, DEPTH10_LEN},
        order::{BookOrder, NULL_ORDER},
        quote::QuoteTick,
        trade::TradeTick,
    },
    enums::{OrderSide, RecordFlag},
    identifiers::TradeId,
    types::{price::Price, quantity::Quantity},
};

use super::{
    parse::{
        parse_aggressor_side, parse_book_action, parse_instrument_id, parse_order_side,
        parse_timestamp,
    },
    record::{
        TardisBookUpdateRecord, TardisOrderBookSnapshot25Record, TardisOrderBookSnapshot5Record,
        TardisQuoteRecord, TardisTradeRecord,
    },
};

/// Creates a new CSV reader which can handle gzip compression.
pub fn create_csv_reader<P: AsRef<Path>>(
    filepath: P,
) -> anyhow::Result<Reader<Box<dyn std::io::Read>>> {
    let file = File::open(filepath.as_ref())?;
    let buf_reader = BufReader::new(file);

    // Determine if the file is gzipped by its extension
    let reader: Box<dyn std::io::Read> =
        if filepath.as_ref().extension().unwrap_or_default() == "gz" {
            Box::new(GzDecoder::new(buf_reader)) // Decompress the gzipped file
        } else {
            Box::new(buf_reader) // Regular file reader
        };

    Ok(ReaderBuilder::new().has_headers(true).from_reader(reader))
}

/// Load [`OrderBookDelta`]s from a Tardis format CSV at the given `filepath`.
pub fn load_deltas<P: AsRef<Path>>(
    filepath: P,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> Result<Vec<OrderBookDelta>, Box<dyn Error>> {
    let mut csv_reader = create_csv_reader(filepath)?;
    let mut deltas: Vec<OrderBookDelta> = Vec::new();
    let mut last_ts_event = UnixNanos::default();

    for result in csv_reader.deserialize() {
        let record: TardisBookUpdateRecord = result?;

        let instrument_id = parse_instrument_id(&record.exchange, &record.symbol);
        let side = parse_order_side(&record.side);
        let price = Price::new(record.price, price_precision);
        let size = Quantity::new(record.amount, size_precision);
        let order_id = 0; // Not applicable for L2 data
        let order = BookOrder::new(side, price, size, order_id);

        let action = parse_book_action(record.is_snapshot, record.amount);
        let flags = 0; // Flags always zero until timestamp changes
        let sequence = 0; // Sequence not available
        let ts_event = parse_timestamp(record.timestamp);
        let ts_init = parse_timestamp(record.local_timestamp);

        // Check if timestamp is different from last timestamp
        if last_ts_event != ts_event {
            if let Some(last_delta) = deltas.last_mut() {
                // Set previous delta flags as F_LAST
                last_delta.flags = RecordFlag::F_LAST.value();
            }
        }

        last_ts_event = ts_event;

        let delta = OrderBookDelta::new(
            instrument_id,
            action,
            order,
            flags,
            sequence,
            ts_event,
            ts_init,
        );

        deltas.push(delta);

        if let Some(limit) = limit {
            if deltas.len() >= limit {
                break;
            }
        }
    }

    // Set F_LAST flag for final delta
    if let Some(last_delta) = deltas.last_mut() {
        last_delta.flags = RecordFlag::F_LAST.value();
    }

    Ok(deltas)
}

fn create_book_order(
    side: OrderSide,
    price: Option<f64>,
    amount: Option<f64>,
    price_precision: u8,
    size_precision: u8,
) -> (BookOrder, u32) {
    match price {
        Some(price) => (
            BookOrder::new(
                side,
                Price::new(price, price_precision),
                Quantity::new(amount.unwrap_or(0.0), size_precision),
                0,
            ),
            1, // Count set to 1 if order exists
        ),
        None => (NULL_ORDER, 0), // NULL_ORDER if price is None
    }
}

/// Load [`OrderBookDepth10`]s from a Tardis format CSV at the given `filepath`.
pub fn load_depth10_from_snapshot5<P: AsRef<Path>>(
    filepath: P,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> Result<Vec<OrderBookDepth10>, Box<dyn Error>> {
    let mut csv_reader = create_csv_reader(filepath)?;
    let mut depths: Vec<OrderBookDepth10> = Vec::new();

    for result in csv_reader.deserialize() {
        let record: TardisOrderBookSnapshot5Record = result?;

        let instrument_id = parse_instrument_id(&record.exchange, &record.symbol);
        let flags = RecordFlag::F_LAST.value();
        let sequence = 0; // Sequence not available
        let ts_event = parse_timestamp(record.timestamp);
        let ts_init = parse_timestamp(record.local_timestamp);

        // Initialize empty arrays
        let mut bids = [NULL_ORDER; DEPTH10_LEN];
        let mut asks = [NULL_ORDER; DEPTH10_LEN];
        let mut bid_counts = [0u32; DEPTH10_LEN];
        let mut ask_counts = [0u32; DEPTH10_LEN];

        for i in 0..=4 {
            // Create bids
            let (bid_order, bid_count) = create_book_order(
                OrderSide::Buy,
                match i {
                    0 => record.bids_0_price,
                    1 => record.bids_1_price,
                    2 => record.bids_2_price,
                    3 => record.bids_3_price,
                    4 => record.bids_4_price,
                    _ => None, // Unreachable, but for safety
                },
                match i {
                    0 => record.bids_0_amount,
                    1 => record.bids_1_amount,
                    2 => record.bids_2_amount,
                    3 => record.bids_3_amount,
                    4 => record.bids_4_amount,
                    _ => None, // Unreachable, but for safety
                },
                price_precision,
                size_precision,
            );
            bids[i] = bid_order;
            bid_counts[i] = bid_count;

            // Create asks
            let (ask_order, ask_count) = create_book_order(
                OrderSide::Sell,
                match i {
                    0 => record.asks_0_price,
                    1 => record.asks_1_price,
                    2 => record.asks_2_price,
                    3 => record.asks_3_price,
                    4 => record.asks_4_price,
                    _ => None, // Unreachable, but for safety
                },
                match i {
                    0 => record.asks_0_amount,
                    1 => record.asks_1_amount,
                    2 => record.asks_2_amount,
                    3 => record.asks_3_amount,
                    4 => record.asks_4_amount,
                    _ => None, // Unreachable, but for safety
                },
                price_precision,
                size_precision,
            );
            asks[i] = ask_order;
            ask_counts[i] = ask_count;
        }

        let depth = OrderBookDepth10::new(
            instrument_id,
            bids,
            asks,
            bid_counts,
            ask_counts,
            flags,
            sequence,
            ts_event,
            ts_init,
        );

        depths.push(depth);

        if let Some(limit) = limit {
            if depths.len() >= limit {
                break;
            }
        }
    }

    Ok(depths)
}

pub fn load_depth10_from_snapshot25<P: AsRef<Path>>(
    filepath: P,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> Result<Vec<OrderBookDepth10>, Box<dyn Error>> {
    let mut csv_reader = create_csv_reader(filepath)?;
    let mut depths: Vec<OrderBookDepth10> = Vec::new();

    for result in csv_reader.deserialize() {
        let record: TardisOrderBookSnapshot25Record = result?;

        let instrument_id = parse_instrument_id(&record.exchange, &record.symbol);
        let flags = RecordFlag::F_LAST.value();
        let sequence = 0; // Sequence not available
        let ts_event = parse_timestamp(record.timestamp);
        let ts_init = parse_timestamp(record.local_timestamp);

        // Initialize empty arrays for the first 10 levels only
        let mut bids = [NULL_ORDER; DEPTH10_LEN];
        let mut asks = [NULL_ORDER; DEPTH10_LEN];
        let mut bid_counts = [0u32; DEPTH10_LEN];
        let mut ask_counts = [0u32; DEPTH10_LEN];

        // Fill only the first 10 levels from the 25-level record
        for i in 0..DEPTH10_LEN {
            // Create bids
            let (bid_order, bid_count) = create_book_order(
                OrderSide::Buy,
                match i {
                    0 => record.bids_0_price,
                    1 => record.bids_1_price,
                    2 => record.bids_2_price,
                    3 => record.bids_3_price,
                    4 => record.bids_4_price,
                    5 => record.bids_5_price,
                    6 => record.bids_6_price,
                    7 => record.bids_7_price,
                    8 => record.bids_8_price,
                    9 => record.bids_9_price,
                    _ => None, // Unreachable, but for safety
                },
                match i {
                    0 => record.bids_0_amount,
                    1 => record.bids_1_amount,
                    2 => record.bids_2_amount,
                    3 => record.bids_3_amount,
                    4 => record.bids_4_amount,
                    5 => record.bids_5_amount,
                    6 => record.bids_6_amount,
                    7 => record.bids_7_amount,
                    8 => record.bids_8_amount,
                    9 => record.bids_9_amount,
                    _ => None, // Unreachable, but for safety
                },
                price_precision,
                size_precision,
            );
            bids[i] = bid_order;
            bid_counts[i] = bid_count;

            // Create asks
            let (ask_order, ask_count) = create_book_order(
                OrderSide::Sell,
                match i {
                    0 => record.asks_0_price,
                    1 => record.asks_1_price,
                    2 => record.asks_2_price,
                    3 => record.asks_3_price,
                    4 => record.asks_4_price,
                    5 => record.asks_5_price,
                    6 => record.asks_6_price,
                    7 => record.asks_7_price,
                    8 => record.asks_8_price,
                    9 => record.asks_9_price,
                    _ => None, // Unreachable, but for safety
                },
                match i {
                    0 => record.asks_0_amount,
                    1 => record.asks_1_amount,
                    2 => record.asks_2_amount,
                    3 => record.asks_3_amount,
                    4 => record.asks_4_amount,
                    5 => record.asks_5_amount,
                    6 => record.asks_6_amount,
                    7 => record.asks_7_amount,
                    8 => record.asks_8_amount,
                    9 => record.asks_9_amount,
                    _ => None, // Unreachable, but for safety
                },
                price_precision,
                size_precision,
            );
            asks[i] = ask_order;
            ask_counts[i] = ask_count;
        }

        let depth = OrderBookDepth10::new(
            instrument_id,
            bids,
            asks,
            bid_counts,
            ask_counts,
            flags,
            sequence,
            ts_event,
            ts_init,
        );

        depths.push(depth);

        if let Some(limit) = limit {
            if depths.len() >= limit {
                break;
            }
        }
    }

    Ok(depths)
}

/// Load [`QuoteTick`]s from a Tardis format CSV at the given `filepath`.
pub fn load_quote_ticks<P: AsRef<Path>>(
    filepath: P,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> Result<Vec<QuoteTick>, Box<dyn Error>> {
    let mut csv_reader = create_csv_reader(filepath)?;
    let mut quotes = Vec::new();

    for result in csv_reader.deserialize() {
        let record: TardisQuoteRecord = result?;

        let instrument_id = parse_instrument_id(&record.exchange, &record.symbol);
        let bid_price = Price::new(record.bid_price.unwrap_or(0.0), price_precision);
        let bid_size = Quantity::new(record.bid_amount.unwrap_or(0.0), size_precision);
        let ask_price = Price::new(record.ask_price.unwrap_or(0.0), price_precision);
        let ask_size = Quantity::new(record.ask_amount.unwrap_or(0.0), size_precision);
        let ts_event = parse_timestamp(record.timestamp);
        let ts_init = parse_timestamp(record.local_timestamp);

        let quote = QuoteTick::new(
            instrument_id,
            bid_price,
            ask_price,
            bid_size,
            ask_size,
            ts_event,
            ts_init,
        );

        quotes.push(quote);

        if let Some(limit) = limit {
            if quotes.len() >= limit {
                break;
            }
        }
    }

    Ok(quotes)
}

/// Load [`TradeTick`]s from a Tardis format CSV at the given `filepath`.
pub fn load_trade_ticks<P: AsRef<Path>>(
    filepath: P,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> Result<Vec<TradeTick>, Box<dyn Error>> {
    let mut csv_reader = create_csv_reader(filepath)?;
    let mut trades = Vec::new();

    for result in csv_reader.deserialize() {
        let record: TardisTradeRecord = result?;

        let instrument_id = parse_instrument_id(&record.exchange, &record.symbol);
        let price = Price::new(record.price, price_precision);
        let size = Quantity::new(record.amount, size_precision);
        let aggressor_side = parse_aggressor_side(&record.side);
        let trade_id = TradeId::new(&record.id);
        let ts_event = parse_timestamp(record.timestamp);
        let ts_init = parse_timestamp(record.local_timestamp);

        let trade = TradeTick::new(
            instrument_id,
            price,
            size,
            aggressor_side,
            trade_id,
            ts_event,
            ts_init,
        );

        trades.push(trade);

        if let Some(limit) = limit {
            if trades.len() >= limit {
                break;
            }
        }
    }

    Ok(trades)
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use nautilus_test_kit::{
        common::{get_project_testdata_path, get_testdata_large_checksums_filepath},
        files::ensure_file_exists_or_download_http,
    };
    use rstest::*;

    use super::*;

    #[rstest]
    pub fn test_read_deltas() {
        let testdata = get_project_testdata_path();
        let checksums = get_testdata_large_checksums_filepath();
        let filename = "tardis_deribit_incremental_book_L2_2020-04-01_BTC-PERPETUAL.csv.gz";
        let filepath = testdata.join("large").join(filename);
        let url = "https://datasets.tardis.dev/v1/deribit/incremental_book_L2/2020/04/01/BTC-PERPETUAL.csv.gz";
        ensure_file_exists_or_download_http(&filepath, url, Some(&checksums)).unwrap();

        let deltas = load_deltas(filepath, 1, 0, Some(1_000)).unwrap();

        assert_eq!(deltas.len(), 1_000)
    }

    #[rstest]
    pub fn test_read_depth10s_from_snapshot5() {
        let testdata = get_project_testdata_path();
        let checksums = get_testdata_large_checksums_filepath();
        let filename = "tardis_binance-futures_book_snapshot_5_2020-09-01_BTCUSDT.csv.gz";
        let filepath = testdata.join("large").join(filename);
        let url = "https://datasets.tardis.dev/v1/binance-futures/book_snapshot_5/2020/09/01/BTCUSDT.csv.gz";
        ensure_file_exists_or_download_http(&filepath, url, Some(&checksums)).unwrap();

        let depths = load_depth10_from_snapshot5(filepath, 1, 0, Some(1_000)).unwrap();

        assert_eq!(depths.len(), 1_000)
    }

    #[rstest]
    pub fn test_read_depth10s_from_snapshot25() {
        let testdata = get_project_testdata_path();
        let checksums = get_testdata_large_checksums_filepath();
        let filename = "tardis_binance-futures_book_snapshot_25_2020-09-01_BTCUSDT.csv.gz";
        let filepath = testdata.join("large").join(filename);
        let url = "https://datasets.tardis.dev/v1/binance-futures/book_snapshot_25/2020/09/01/BTCUSDT.csv.gz";
        ensure_file_exists_or_download_http(&filepath, url, Some(&checksums)).unwrap();

        let depths = load_depth10_from_snapshot25(filepath, 1, 0, Some(1_000)).unwrap();

        assert_eq!(depths.len(), 1_000)
    }

    #[rstest]
    pub fn test_read_quotes() {
        let testdata = get_project_testdata_path();
        let checksums = get_testdata_large_checksums_filepath();
        let filename = "tardis_huobi-dm-swap_quotes_2020-05-01_BTC-USD.csv.gz";
        let filepath = testdata.join("large").join(filename);
        let url = "https://datasets.tardis.dev/v1/huobi-dm-swap/quotes/2020/05/01/BTC-USD.csv.gz";
        ensure_file_exists_or_download_http(&filepath, url, Some(&checksums)).unwrap();

        let quotes = load_quote_ticks(filepath, 1, 0, Some(1_000)).unwrap();

        assert_eq!(quotes.len(), 1_000)
    }

    #[rstest]
    pub fn test_read_trades() {
        let testdata = get_project_testdata_path();
        let checksums = get_testdata_large_checksums_filepath();
        let filename = "tardis_bitmex_trades_2020-03-01_XBTUSD.csv.gz";
        let filepath = testdata.join("large").join(filename);
        let url = "https://datasets.tardis.dev/v1/bitmex/trades/2020/03/01/XBTUSD.csv.gz";
        ensure_file_exists_or_download_http(&filepath, url, Some(&checksums)).unwrap();

        let trades = load_trade_ticks(filepath, 1, 0, Some(1_000)).unwrap();

        assert_eq!(trades.len(), 1_000)
    }
}
