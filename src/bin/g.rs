use crawlins::webster::gen_anki;
#[tokio::main]
async fn main() {
    gen_anki().await.unwrap();
}
