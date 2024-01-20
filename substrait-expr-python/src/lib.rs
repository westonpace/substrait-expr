use pyo3::prelude::*;

#[pyfunction]
fn guess_the_number() {
    println!("Guess the number!");
}

#[pymodule]
fn _internal(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(guess_the_number, m)?)?;
    Ok(())
}
