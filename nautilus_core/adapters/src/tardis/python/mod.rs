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

//! Python bindings from `pyo3`.

#![allow(warnings)] // non-local `impl` definition, temporary allow until pyo3 upgrade

pub mod load;

use pyo3::prelude::*;

/// Loaded as nautilus_pyo3.tardis
#[pymodule]
pub fn tardis(_: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load::py_load_tardis_deltas, m)?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_deltas_as_pycapsule,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_depth10_from_snapshot5,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_depth10_from_snapshot5_as_pycapsule,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_depth10_from_snapshot25,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_depth10_from_snapshot25_as_pycapsule,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(load::py_load_tardis_quotes, m)?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_quotes_as_pycapsule,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(load::py_load_tardis_trades, m)?)?;
    m.add_function(wrap_pyfunction!(
        load::py_load_tardis_trades_as_pycapsule,
        m
    )?)?;
    Ok(())
}
