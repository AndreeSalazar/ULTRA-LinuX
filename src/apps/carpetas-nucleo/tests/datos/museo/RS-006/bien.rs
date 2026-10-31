pub fn numero(s: &str) -> Result<u32, core::num::ParseIntError> {
    s.parse()
}

#[cfg(test)]
mod pruebas {
    #[test]
    fn lee() {
        assert_eq!(super::numero("3").unwrap(), 3);
    }
}
