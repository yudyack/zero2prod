use std::fmt::{Debug, Display};

use tokio::task::JoinError;
// this binary will target "package name"
use zero2prod::configuration::get_configuration;
use zero2prod::issue_delivery_worker::run_worker_until_stopped;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber =
        get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration =
        get_configuration().expect("failed to read configuration ");

    let application = Application::build(configuration.clone()).await?;
    let application = application.run_until_stopped();
    let worker = run_worker_until_stopped(configuration);

    let application_task = tokio::spawn(application);
    let worker_task = tokio::spawn(worker);
    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Backgground worker", o),
    };

    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl Debug + Display>, JoinError>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error! {
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed", task_name
            }
        }
        Err(e) => {
            tracing::error! {
                error.cause_chain = ?e,
                error.message = %e,
                "{} task failed to complete", task_name
            }
        }
    }
}
