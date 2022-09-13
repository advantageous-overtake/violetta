mod test;

#[allow(dead_code)]
fn greet_user( user: &str ) -> String {
    format!( "Hello, {user}!" )
}

fn main() {
    println!("Hello, world!");
}
