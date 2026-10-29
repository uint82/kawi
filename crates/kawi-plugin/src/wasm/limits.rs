use wasmtime::ResourceLimiter;

const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;
const MAX_TABLE_ELEMENTS: usize = 1024 * 1024;

pub struct WasmLimits {
    memory_bytes: usize,
    table_elements: usize,
}

impl WasmLimits {
    pub fn new() -> Self {
        Self {
            memory_bytes: 0,
            table_elements: 0,
        }
    }
}

impl Default for WasmLimits {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceLimiter for WasmLimits {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let allowed =
            desired <= MAX_MEMORY_BYTES && maximum.is_none_or(|maximum| desired <= maximum);
        if allowed {
            self.memory_bytes = desired;
        }
        Ok(allowed)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let allowed =
            desired <= MAX_TABLE_ELEMENTS && maximum.is_none_or(|maximum| desired <= maximum);
        if allowed {
            self.table_elements = desired;
        }
        Ok(allowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_memory_above_the_limit() {
        let mut limits = WasmLimits::new();
        assert!(!limits
            .memory_growing(0, MAX_MEMORY_BYTES + 1, None)
            .unwrap());
        assert!(limits.memory_growing(0, MAX_MEMORY_BYTES, None).unwrap());
    }

    #[test]
    fn rejects_tables_above_the_limit() {
        let mut limits = WasmLimits::new();
        assert!(!limits
            .table_growing(0, MAX_TABLE_ELEMENTS + 1, None)
            .unwrap());
        assert!(limits.table_growing(0, MAX_TABLE_ELEMENTS, None).unwrap());
    }
}
