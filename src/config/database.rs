use dotenv::dotenv;
use std::env;

pub fn get_database_url() -> String {
    // .envファイルを読み込む
    dotenv().ok();

    // DATABASE_URLを直接取得
    match env::var("DATABASE_URL") {
        Ok(url) => {
            // 環境変数内の変数展開を手動で行う
            let user = env::var("MYSQL_USER").unwrap_or_default();
            let password = env::var("MYSQL_PASSWORD").unwrap_or_default();
            let port = env::var("MYSQL_PORT").unwrap_or_default();
            let host = env::var("MYSQL_HOST").unwrap_or_default();
            let database = env::var("MYSQL_DATABASE").unwrap_or_default();

            let url = url
                .replace("${MYSQL_USER}", &user)
                .replace("${MYSQL_PASSWORD}", &password)
                .replace("${MYSQL_PORT}", &port)
                .replace("${MYSQL_HOST}", &host)
                .replace("${MYSQL_DATABASE}", &database);

            url
        }
        Err(_) => {
            // DATABASE_URLが設定されていない場合は手動で構築
            let user = env::var("MYSQL_USER").unwrap_or_default();
            let password = env::var("MYSQL_PASSWORD").unwrap_or_default();
            let port = env::var("MYSQL_PORT").unwrap_or_default();
            let database = env::var("MYSQL_DATABASE").unwrap_or_default();

            format!(
                "mysql://{}:{}@localhost:{}/{}",
                user, password, port, database
            )
        }
    }
}
