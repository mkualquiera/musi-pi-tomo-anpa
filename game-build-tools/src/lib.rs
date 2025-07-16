pub mod level;

macro_rules! build_log {
    ($($arg:tt)*) => {
        println!("cargo:warning=[BUILD] {}", format!($($arg)*));
    };
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
