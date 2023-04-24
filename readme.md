# Mailer

A simple email sending micro service, made for asynchronous and event based systems with Rabbitmq and AWS SES.

## Architecture

![diagram](./docs/diagram.png "diagram")

The service declares and consumes a single persistent queue, producers can send to the queue using a direct exchange or by declaring their own exchanges  
and binding them to said queue, although this would require the mailer queue to be declared beforehand.

this service declares and publishes events to a exchange so consumers can receive events such as when a email was sent, clicked, reported, etc.

## Environment variables

|           name                    |                                    meaning                         | example                           |
|-----------------------------------|--------------------------------------------------------------------|-----------------------------------|
| APP_DEBUG                         | debug mode, if true will log to debug info to stdout               | false                             |
| APP_DEFAULT_EMAIL_SENDER          | default email address to be used as the sender                     | no-reply@your-company.com         |
|                                   |                                                                    |                                   |
| RMQ_URI                           | rabbitmq url                                                       | amqp://guest:guest@localhost:5672 |
| RMQ_QUEUE                         | name of the rabbitmq queue to listen for messages                  | mailer_queue                      |
| RMQ_CONSUMER_TAG                  | name of the consumer tag for the queue consumer                    | mailer_queue_consumer             |
| RMQ_EMAIL_EVENTS_EXCHANGE         | name for the exchange to publish email events on                   | mailer_events                     |
|                                   |                                                                    |                                   |
| AWS_REGION                        |                                                                    | us-east-1                         |
| AWS_SES_TRACKING_CONFIG_SET       | name of the SES configuration set to use for email tracking        | track-all-events                  |
| AWS_SES_MAX_EMAILS_PER_SECOND     | limit for ops/s for the SES send email operation for your account  | 1                                 |
| AWS_SNS_TRACKING_SUBSCRIPTION_ARN | AWS ARN for the SNS subscription for the email tracking config set | arn:123...                        |
|                                   |                                                                    |                                   |
| TRACER_SERVICE_NAME               | name of the service to jaeger                                      | mailer                            |
| HTTP_PORT                         | HTTP port to listen on for SNS events                              | 3005                              |

## Known limitations

- SES Rate limiting for multiple instances of this service:

rate limiting for SES sendEmail operations is done in memory, this means rate limiting between multiple instances
of this service using the same SES account wont work and you should implement rate limiting that is shared by those
instances (eg: redis). This should not be a problem since this service is a lightweight wrapper around SES and a 
single instance should be more than enough.

### TODO (to release v1.0)
- document dev setup with ngrok
- document aws ses / sns, rabbitmq setup
- document jaeger and jaeger setup
- document DTOS
- finish all email events and code todo`s
- create a logo ?
- get rid of junk spans on jaeger (trace level ?)
