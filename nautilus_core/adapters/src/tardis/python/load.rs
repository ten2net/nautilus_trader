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

use std::path::PathBuf;

use nautilus_core::{ffi::cvec::CVec, python::to_pyvalue_err};
use nautilus_model::data::{
    delta::OrderBookDelta, depth::OrderBookDepth10, quote::QuoteTick, trade::TradeTick,
};
use pyo3::{prelude::*, types::PyCapsule};

use crate::tardis::load::{
    load_deltas, load_depth10_from_snapshot25, load_depth10_from_snapshot5, load_quote_ticks,
    load_trade_ticks,
};

#[must_use]
#[pyfunction(name = "load_tardis_deltas")]
pub fn py_load_tardis_deltas(
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<Vec<OrderBookDelta>> {
    load_deltas(filepath, price_precision, size_precision, limit).map_err(to_pyvalue_err)
}

#[must_use]
#[pyfunction(name = "load_tardis_deltas_as_pycapsule")]
pub fn py_load_tardis_deltas_as_pycapsule(
    py: Python,
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let deltas =
        load_deltas(filepath, price_precision, size_precision, limit).map_err(to_pyvalue_err)?;

    let cvec: CVec = deltas.into();
    let capsule = PyCapsule::new_bound::<CVec>(py, cvec, None)?;
    Ok(capsule.into_py(py))
}

#[must_use]
#[pyfunction(name = "load_tardis_depth10_from_snapshot5")]
pub fn py_load_tardis_depth10_from_snapshot5(
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<Vec<OrderBookDepth10>> {
    load_depth10_from_snapshot5(filepath, price_precision, size_precision, limit)
        .map_err(to_pyvalue_err)
}

#[must_use]
#[pyfunction(name = "load_tardis_depth10_from_snapshot5_as_pycapsule")]
pub fn py_load_tardis_depth10_from_snapshot5_as_pycapsule(
    py: Python,
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let depths = load_depth10_from_snapshot5(filepath, price_precision, size_precision, limit)
        .map_err(to_pyvalue_err)?;

    let cvec: CVec = depths.into();
    let capsule = PyCapsule::new_bound::<CVec>(py, cvec, None)?;
    Ok(capsule.into_py(py))
}

#[must_use]
#[pyfunction(name = "load_tardis_depth10_from_snapshot25")]
pub fn py_load_tardis_depth10_from_snapshot25(
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<Vec<OrderBookDepth10>> {
    load_depth10_from_snapshot25(filepath, price_precision, size_precision, limit)
        .map_err(to_pyvalue_err)
}

#[must_use]
#[pyfunction(name = "load_tardis_depth10_from_snapshot25_as_pycapsule")]
pub fn py_load_tardis_depth10_from_snapshot25_as_pycapsule(
    py: Python,
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let depths = load_depth10_from_snapshot25(filepath, price_precision, size_precision, limit)
        .map_err(to_pyvalue_err)?;

    let cvec: CVec = depths.into();
    let capsule = PyCapsule::new_bound::<CVec>(py, cvec, None)?;
    Ok(capsule.into_py(py))
}

#[must_use]
#[pyfunction(name = "load_tardis_quotes")]
pub fn py_load_tardis_quotes(
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<Vec<QuoteTick>> {
    load_quote_ticks(filepath, price_precision, size_precision, limit).map_err(to_pyvalue_err)
}

#[must_use]
#[pyfunction(name = "load_tardis_quotes_as_pycapsule")]
pub fn py_load_tardis_quotes_as_pycapsule(
    py: Python,
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let quotes = load_quote_ticks(filepath, price_precision, size_precision, limit)
        .map_err(to_pyvalue_err)?;

    let cvec: CVec = quotes.into();
    let capsule = PyCapsule::new_bound::<CVec>(py, cvec, None)?;
    Ok(capsule.into_py(py))
}

#[must_use]
#[pyfunction(name = "load_tardis_trades")]
pub fn py_load_tardis_trades(
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<Vec<TradeTick>> {
    load_trade_ticks(filepath, price_precision, size_precision, limit).map_err(to_pyvalue_err)
}

#[must_use]
#[pyfunction(name = "load_tardis_trades_as_pycapsule")]
pub fn py_load_tardis_trades_as_pycapsule(
    py: Python,
    filepath: PathBuf,
    price_precision: u8,
    size_precision: u8,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let trades = load_trade_ticks(filepath, price_precision, size_precision, limit)
        .map_err(to_pyvalue_err)?;

    let cvec: CVec = trades.into();
    let capsule = PyCapsule::new_bound::<CVec>(py, cvec, None)?;
    Ok(capsule.into_py(py))
}
