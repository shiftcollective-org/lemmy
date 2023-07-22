# Lemmy - Breakout (v2)

This is a breakout of Lemmy into Scheduler, Federation and Api components.

## Running on Docker Compose

First: `export LEMMY_DATABASE_URL=postgres://lemmy:password@localhost:5433/lemmy`

To build and run the docker-compose file:

```Bash
cd docker
./docker_update.sh
```

## Tests

Run Rust unit tests:

```Bash
./scripts/test.sh
```

## Components

### Lemmy Api (port 8536 by default)

* Exposes API endpoints for the Lemmy-UI frontend and any other clients (all routes starting with /api)
* Exposes RSS/Atom feeds
* Exposes nodeinfo endpoint for discovery (crates/routes/src/nodeinfo.rs)
* Proxies image requests to pictrs (crates/routes/src/images.rs)
  * Handle routes /image

### Lemmy Federation (port 8537 by default)

* Manages ActivityPub messages going out and into the server from/to other federated servers

* Manages community profiles, followers and outboxes.
  * Handle routes:
    * /c/{community_name}
    * /c/{community_name}/followers
    * /c/{community_name}/outbox
    * /c/{community_name}/featured
    * /c/{community_name}/moderators

* Handles the overall site representation and outbox.
  * Handle routes:
    * /
    * /site_outbox

* Manages user profiles, followers and outboxes.
  * Handle routes:
    * /u/{user_name}
    * /u/{user_name}/outbox

* Manages posts
  * Handle route:
    * /post/{post_id}

* Manages comments on posts
  * Handle route:
    * /comment/{comment_id}

* Receives and processes incoming activities.
  * Handle route:
    * /activities/{type_}/{id}

* Handles all incoming activitypub requests
  * Handle routes:
    * /c/{community_name}/inbox
    * /u/{user_name}/inbox
    * /inbox
    * /site_inbox

* Handles webfinger endpoint
  * Handle route:
    * .well-known/webfinger

### Lemmy Scheduler

* Handles Scheduled Tasks
* Handles DB Migrations on startup

## Considerations

The key considerations are:

* This structure allows independent scaling of the api and federation service - which will handle most of the traffic and allow the Lemmy service to only handle scheduled tasks (and db migrations)
* For getting started, the simplest approach was used: switching the main lemmy workspace to a library, and breaking out the minimum amount of services that would provide value
* The new packages all reference the same shared library (the main lemmy code) and each use parts of it relevant to their use case
* Kept everything inside the same repository, sharing the same common library, for safety and convenience of integrating upstream updates more easily - all upstream updates can be easily synced with the common library as the code did not change - in case of conflicts, they will be easy to resolve. The split is only at the service level, not at the library level.
* Building the docker images will be done using multi-stage builds, compiling the shared library in one stage, and then using it in the subsequent stages for the different services. This ensures the compiled library is cached and reused.
* All unit tests will still work as they are in the common library.
