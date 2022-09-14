use rocket::{
    data::ToByteUnit,
    fairing::AdHoc,
    figment::{
        providers::{Env, Format, Serialized, Toml},
        Figment, Profile,
    },
    fs::{relative, NamedFile},
    http::{ContentType, Status},
    request::{FromRequest, Outcome},
    serde::json::Json,
    serde::*,
    tokio::{fs::File, io::AsyncWriteExt},
    *,
};
use uuid::Uuid;

#[derive(Default)]
struct RequiresAuthentication;

#[rocket::async_trait]
impl<'a> FromRequest<'a> for RequiresAuthentication {
    type Error = UploadResponse;

    async fn from_request(req: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        let expected_key = &req
            .guard::<&State<UploaderConfiguration>>()
            .await
            .unwrap()
            .inner()
            .auth_key;
        match expected_key {
            Some(actual_key @ _) => {
                if let Some(extern_key) = req.headers().get_one("Authorization") {
                    if actual_key == extern_key {
                        Outcome::Success(RequiresAuthentication::default())
                    } else {
                        Outcome::Failure((Status::Unauthorized, UploadResponse::error()))
                    }
                } else {
                    Outcome::Failure((Status::Unauthorized, UploadResponse::error()))
                }
            }
            None => Outcome::Success(RequiresAuthentication::default()),
        }
    }
}

#[derive(Deserialize)]
struct UploaderConfiguration {
    pub max_size: u32,
    pub auth_key: Option<String>,
    pub allowed_extensions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UploadResponse {
    successful: bool,
    file_id: Option<String>,
}

impl UploadResponse {
    fn new(successful: bool, file_id: Option<String>) -> Self {
        Self {
            successful,
            file_id,
        }
    }

    fn error() -> Self {
        Self {
            successful: false,
            file_id: None,
        }
    }
}

#[put("/", data = "<file>")]
async fn upload_media(
    file: Data<'_>,
    config: &State<UploaderConfiguration>,
    _a: RequiresAuthentication,
    content_type: &ContentType,
) -> std::io::Result<Json<UploadResponse>> {
    match content_type.extension().map(|ext| ext.as_str()) {
        ext @ Some("png") | ext @ Some("mp4") => {
            let ext = ext.unwrap();
            let current_dir = std::env::current_dir().unwrap();
            let raw_data = file
                .open(config.max_size.mebibytes())
                .into_bytes()
                .await?
                .into_inner();
            let file_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, &raw_data);
            let mut file = File::create(current_dir.join(format!(
                "{}/{}.{ext}",
                relative!("images"),
                file_id
            )))
            .await?;
            file.write_all(&raw_data.as_slice()).await?;
            Ok(Json(UploadResponse::new(true, Some(file_id.to_string()))))
        }
        _ => Ok(Json(UploadResponse::error())),
    }
}

#[get("/<file_id>")]
async fn retrieve_media(file_id: &str, config: &State<UploaderConfiguration>) -> Option<NamedFile> {
    let current_dir = std::env::current_dir().unwrap();
    let ext: Option<String> = {
        let mut corresponding_ext: Option<String> = None;
        for ext in config.allowed_extensions.iter() {
            let corresponding_path =
                current_dir.join(format!("{}/{}.{ext}", relative!("images"), file_id));

            if corresponding_path.is_file() {
                corresponding_ext = Some(ext.clone());
            }
        }
        corresponding_ext
    };
    if let Some(ext) = ext {
        Some(
            NamedFile::open(current_dir.join(format!("{}/{file_id}.{ext}", relative!("images"))))
                .await
                .unwrap(),
        )
    } else {
        None
    }
}

#[get("/")]
fn index() -> &'static str {
    "USAGE
        GET /
            what are you reading right now
    
        PUT /
            upload a file, send as raw body

        GET /:id
            gets the file <:id>
    "
}

#[catch(401)]
fn unauthorized() -> Json<UploadResponse> {
    Json(UploadResponse::error())
}

#[catch(500)]
fn internal_server_err() -> Json<UploadResponse> {
    Json(UploadResponse::error())
}

#[catch(404)]
fn not_found() -> Json<UploadResponse> {
    Json(UploadResponse::error())
}

#[launch]
pub fn rocket() -> _ {
    rocket::custom(
        Figment::from(rocket::Config::default())
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("violetta.toml").nested())
            .merge(Env::prefixed("VIOLETTA_").global())
            .select(Profile::from_env_or("VIOLETTA_PROFILE", "default")),
    )
    .mount("/", routes![index, upload_media, retrieve_media])
    .register("/", catchers![not_found, unauthorized, internal_server_err])
    .attach(AdHoc::config::<UploaderConfiguration>())
}


#[cfg(test)]
mod test {
    use crate::*;
    use rocket::local::blocking::Client;

    macro_rules! client {
        () => {
            Client::tracked(rocket()).expect("valid rocket instance")
        };
    }

    macro_rules! request {
        ( $client:ident, $method:ident, $path:expr ) => {
            $client.$method($path)
        };
    }

    #[test]
    fn upload_file() {
        let client = client!();
        let response = request!(client, put, "/")
            .header(ContentType::PNG)
            .body(&[0u8; 1024])
            .dispatch();
        let response_body = response.into_json::<UploadResponse>().unwrap();
        assert_eq!(response_body.successful, true);
        assert!(response_body.file_id.is_some());
        let response = request!(client, get, format!( "/{}", response_body.file_id.unwrap() )).dispatch();
        assert_eq!(response.into_bytes().unwrap().as_slice(), [0u8; 1024]);
    }
}
