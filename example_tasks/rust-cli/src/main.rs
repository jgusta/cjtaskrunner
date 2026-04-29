use std::env;

fn main() {
    let greeting = env::var("APP_GREETING").unwrap_or_else(|_| "hello".to_string());
    println!("{greeting} from Rust");
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic_math_still_works() {
        assert_eq!(2 + 2, 4);
    }
}
