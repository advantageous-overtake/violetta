#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn does_it_greet_us( ) {
        assert_eq!( greet_user( "John" ), "Hello, John!" );
    }
}