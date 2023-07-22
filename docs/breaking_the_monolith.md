# Breaking up Lemmy into Microservices (Initial Version)

## Breakout

See docker/docker-compose.yml and docker/nginx.conf for how to setup.

### API Service (port 8538 by default)

- Contains all the routes previously from src/api_routes_http.rs (all routes starting with /api)
- Exposes API endpoints for the Lemmy-UI frontend and any other clients
- Exposes RSS/Atom feeds
- Handles nodeinfo endpoint for discovery.
  - Contains routes from services/api_service/src/nodeinfo.rs
- Proxies image requests to pictrs (crates/routes/src/images.rs)
  - Handle routes /image

### Federation Service (port 8537 by default)

- Manages ActivityPub messages going out and into the server from/to other federated servers

- Manages community profiles, followers and outboxes.
  - Handle routes:
    - /c/{community_name}
    - /c/{community_name}/followers
    - /c/{community_name}/outbox
    - /c/{community_name}/featured
    - /c/{community_name}/moderators

- Handles the overall site representation and outbox.
  - Handle routes:
    - /
    - /site_outbox

- Manages user profiles, followers and outboxes.
  - Handle routes:
    - /u/{user_name}
    - /u/{user_name}/outbox

- Manages posts
  - Handle route:
    - /post/{post_id}

- Manages comments on posts
  - Handle route:
    - /comment/{comment_id}

- Receives and processes incoming activities.
  - Handle route:
    - /activities/{type_}/{id}

- Handles all incoming activitypub requests
  - Handle routes:
    - /c/{community_name}/inbox
    - /u/{user_name}/inbox
    - /inbox
    - /site_inbox

- Handles webfinger endpoint
  - Handle route:
    - .well-known/webfinger

### Lemmy

- Handles Scheduled Tasks
- Handles DB Migrations

### Considerations

The key considerations are:

- This structure allows independent scaling of the api and federation service - which will handle most of the traffic and allow the Lemmy service to mostly handle scheduled tasks
- For getting started, the simplest approach was used: using a shared Postgres database, having services share the common database, and breaking out the minimum amount of services that would provide value
- Kept everything inside the same repository, for safety and convenience of integrating upstream updates more easily
- Referenced common crates from the new services (by referencing the git repo - because if referencing paths, we could not build docker images) from the common source code instead of copying everything into each service - again because of convenience
- As the system grows past the PoC, it may make sense to break more out of the monoliths and separate the services into different repositories, as well as clean them up
- The assumptions is that an automated process/tool can be built to handle integrating changes across services whenever there's an official lemmy update (this should be easier with all services in the same repository and shared common code)
- A shared database is fine for a PoC, but production systems should likely break out into separate databases per service, and use events/queue to propagate changes between services.
