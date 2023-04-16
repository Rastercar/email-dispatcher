# Mailer

A simple email sending micro service, made for asynchonous and event based systems with rabbitmq and AWS SES.

## Architecture

![diagramn](./docs/diagram.png "diagram")

The service declares and consumes a single persistent queue, producers can send to the queue using a direct exchange or by declaring their own exchanges  
and binding them to said queue, although this would require the mailer queue to be declared beforehand.

this service declares and publishes events to a exchange so consumers can recieve events such as when a email was sent, clicked, reported, etc.

## Datastore

A postgres db is used to store the sent emails and their events

### TODO (to release v1.0)
- integrate with aws ses rust sdk
- document dev setup
- docker and docker docs
- document required postgres, aws ses, rabbitmq setup
- create a example consumer repository
- create a logo ?
- document jaeger and jaeger setup
- finish basic functionality
- document DTOS (async api ?)