use std::fs;

pub struct DatabaseConfig {
    pub host: String,
    pub port: String,
    pub db: String,
    pub user: String,
    pub pwd: String,
}

impl DatabaseConfig {
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.pwd, self.host, self.port, self.db
        )
    }
}

pub fn load_database_configuration() -> Option<DatabaseConfig> {
    let host = dotenvy::var("PSQL_HOST").expect("PSQL_HOST not found");
    let port = dotenvy::var("PSQL_PORT").expect("PSQL_PORT not found");

    let user = match dotenvy::var("PSQL_USER_FILE") {
        Ok(path) => fs::read_to_string(path).expect("Error while trying to read secret: USER"),
        Err(_) => dotenvy::var("PSQL_USER").expect("No PSQL_USER_FILE nor PSQL_USER"),
    };

    let pwd = match dotenvy::var("PSQL_PASSWORD_FILE") {
        Ok(path) => fs::read_to_string(path).expect("Error while trying to read secret: PWD"),
        Err(_) => dotenvy::var("PSQL_PASSWORD").expect("No PSQL_PASSWORD_FILE nor PSQL_PWD"),
    };

    let db = match dotenvy::var("PSQL_DB_FILE") {
        Ok(path) => fs::read_to_string(path).expect("Error while trying to read secret: DB"),
        Err(_) => dotenvy::var("PSQL_DB").expect("No PSQL_DB_FILE nor PSQL_DB"),
    };

    Some(DatabaseConfig {
        host,
        port,
        db,
        user,
        pwd,
    })
}
