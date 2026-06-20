use reqwest::{Client, redirect::Policy};

#[tokio::main]
async fn main() {
    let client = Client::builder().redirect(Policy::none()).build().unwrap();
    let resp = client.get("http://httpbin.org/redirect-to?url=http%3A%2F%2F169.254.169.254%2F").send().await.unwrap();
    println!("{:?} {:?}", resp.status(), resp.headers().get("location"));
}
