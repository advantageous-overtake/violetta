mod test;

use rocket::*;

#[get("/hello/<name>")]
fn greet_user( name: String ) -> String {
    format!("Hello, {name}!")
}

#[get("/")]
fn index( ) -> &'static str {
    "USAGE
        GET /hello/:name
            simply, greets you.
    "
}

#[launch]
pub fn rocket() -> _ {
    rocket::build()
        .mount(
            "/", routes![ index, greet_user ]
        )
}
