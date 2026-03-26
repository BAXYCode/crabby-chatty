# API implementation

## Current
    Currently I am implementing the api in a very amateur way by having requests
    include the api caller as the "actor_id" in the request body. 

## Planned
    Once I have implemented my auth flow correctly (ForwardaAuth from traefik to
    auth service) I will be moving to a more traditional implementation with the
    caller's information being stored in HTTP headers. 

    This architecture will be cleaner and more up to par with the standards
    expected in  production environments

# API spec generation

## Planned

    I need to find a more ergonomic way to change the necessary information in
    the api spec generation. I would like to build some reusable module that
    would allow all my microservices to generate the appropriate spec with the
    correct information from a config file
