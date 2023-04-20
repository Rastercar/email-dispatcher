# Mailer

A simple email sending micro service, made for asynchonous and event based systems with rabbitmq and AWS SES.

## Architecture

![diagramn](./docs/diagram.png "diagram")

The service declares and consumes a single persistent queue, producers can send to the queue using a direct exchange or by declaring their own exchanges  
and binding them to said queue, although this would require the mailer queue to be declared beforehand.

this service declares and publishes events to a exchange so consumers can recieve events such as when a email was sent, clicked, reported, etc.

### TODO (to release v1.0)
- docker and docker docs
- document dev setup with ngrok
- document aws ses, rabbitmq setup
- document jaeger and jaeger setup
- document DTOS (async api ?)
- remove postgres from readme diagram
- create a logo ?
- create a example consumer repository
- finish basic functionality
- gracefull shutdown
- document the need for rate limiting if using more pods