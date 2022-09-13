#[cfg(test)]
mod test {
    use rocket::local::blocking::Client;
    use crate::*;

    #[test]
    fn does_it_greet_us( ) {
        let client = Client::tracked( rocket() )
            .expect( "invalid rocket instance" );

        let response = client.get( "/hello/World" ).dispatch( );
        assert_eq!( response.into_string( ).unwrap( ), "Hello, World!" );
    }
}