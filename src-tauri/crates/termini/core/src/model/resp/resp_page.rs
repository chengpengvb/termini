use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct QueryPage<T> {
    pub total: usize,
    pub rows: Vec<T>,
    pub page_size: usize,
    pub page_number: usize,
}

impl<T> QueryPage<T> {
    pub fn new(page_number: usize, page_size: usize, all_rows: Vec<T>) -> Self {
        let total = all_rows.len();
        let start = page_number * page_size;
        let rows = all_rows.into_iter().skip(start).take(page_size).collect();
        Self { total, page_number, page_size, rows }
    }
}
