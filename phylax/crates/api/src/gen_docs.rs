use phylax_api::docs::ApiDoc;
use std::fs;
use utoipa::OpenApi;
// in ./src/gen_openapi.rs
fn main() {
    let doc = gen_my_openapi();
    fs::write("./open-api.json", doc).expect("Failed to generate api docs");
}

// in /src/openapi.rs
fn gen_my_openapi() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}
