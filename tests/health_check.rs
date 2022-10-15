use sqlx::{Connection, PgConnection};
use std::net::TcpListener;

use newsletter_api::configuration::get_configuration;
use newsletter_api::startup::run;

// launch our application in the background
fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let server = run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)
#[tokio::test]
async fn health_check_works() {
    // arrange
    let address = spawn_app();

    // perform HTTP request against our application
    let client = reqwest::Client::new();

    // act
    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute request");

    // assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let address = spawn_app();

    let configuration = get_configuration().expect("Failed to read configuration");
    let connection_string = configuration.database.connection_string();

    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres.");

    let client = reqwest::Client::new();

    // act
    let body = "name=nik&email=nikhil%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    // assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "nikhil@gmail.com");
    assert_eq!(saved.name, "nik");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // arrange
    let address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=nik", "missing the email"),
        ("email=nikhil%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    // act
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(), // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
