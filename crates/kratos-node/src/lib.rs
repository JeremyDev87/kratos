use napi_derive::napi;

#[napi(js_name = "runCli")]
pub fn run_cli(args: Vec<String>) -> i32 {
    kratos_cli::run_cli(&args)
}
