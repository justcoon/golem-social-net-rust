# golem-social-net-rust

## Overview
This is a simple social net application built with an agent-based architecture, leveraging Golem Cloud for distributed, stateful agents.

![Architecture Diagram](architecture.png)

*Figure 1: Golem Social Net Application Architecture*

## Architecture Components

### Agents
The application follows a granular agent-based architecture, where different aspects of the system are managed by specialized Golem agents:

#### Stateful Agents (Persistent)
- **User Agent**: Manages user profile information (name, email) and maintains a list of connections (friends and followers).
- **Post Agent**: Manages the lifecycle of an individual post, including its content, likes, and a hierarchical comment system.
- **User Post Agent**: Maintains a registry of all posts created by a specific user.
- **User Timeline Agent**: Stores references to posts that should appear in a user's personal timeline.
- **Timelines Updater Agent**: Orchestrates the distribution of new posts to the timelines of the author and their connections.

#### Ephemeral Agents (View/Computational)
- **User Search Agent**: Performs global user searches by discovering and querying User Agent instances.
- **User Posts View Agent**: Generates a detailed view of a user's posts by aggregating content from multiple Post Agents.
- **User Timeline View Agent**: Generates a detailed view of a user's timeline by aggregating content from multiple Post Agents.
- **User Timeline Updates Agent**: Implements a long-polling mechanism to provide real-time updates for a user's timeline.

### Key Features
- **RESTful API** for all social net operations
- **Stateful Agents** with Golem Cloud managing the state
- **Distributed Agent System** with clear responsibility boundaries

### Communication Flow
The system manages interactions through several distinct flow patterns:

1. **Request Entry & Routing**:
   - The **API Gateway** acts as the entry point, receiving HTTP REST requests from the frontend.
   - It maps these requests to specific **Golem RPC** calls targeting the appropriate agent (e.g., `/users/{id}` maps to a `User Agent`).

2. **Discovery & Search**:
   - **User Search Agent** (ephemeral) discovers stateful **User Agents** by querying Golem's metadata and filtering by name patterns.
   - Once a subset of agents is found, it performs parallel RPC calls to retrieve matching profile data.

3. **Content Aggregation (Materialized Views)**:
   - **View Agents** (User Posts View, User Timeline View) handle complex read operations.
   - They first retrieve a list of post references (IDs and timestamps) from stateful registry agents (**User Post Agent** or **User Timeline Agent**).
   - They then resolve these references by fetching full content, likes, and comments from multiple **Post Agents** in parallel.

4. **Event-Driven Post Distribution (Fan-out)**:
   - When a **Post Agent** is initialized (created), it triggers a "Post Created" event.
   - The **Timelines Updater Agent** receives this event and identifies the author's connections via the **User Agent**.
   - It then performs a fan-out broadcast, adding the post reference to the **User Timeline Agent** of every follower and friend.

5. **Real-time Synchronization**:
   - The **User Timeline Updates Agent** implements a long-polling mechanism.
   - It monitors the state of a **User Timeline Agent** and returns new post references as soon as they are broadcased by the updater, allowing for live feed updates.

### State Management
All core agents (User, Post, User Posts, User Timeline) have their state managed by Golem Cloud, ensuring reliability and scalability through the agent-based architecture.


## Quick Start

1. **Prerequisites**:
    - Install [Golem CLI](https://learn.golem.cloud/cli) (version 1.4.0+)
    - [Running Golem Environment](https://learn.golem.cloud/quickstart#running-golem)

   See [Golem Quickstart](https://learn.golem.cloud/quickstart) for more information.


2. **Build and Deploy**:
   ```bash
   # Build all components
   golem-cli build
   
   # Deploy to Golem
   golem-cli deploy
   ```

3. **Import Sample Data**:
   For information on importing sample data, see the [Data README](./data/README.md).

