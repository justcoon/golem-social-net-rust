# golem-social-net-rust

## Overview
This is a simple social net application built with an agent-based architecture, leveraging Golem Cloud for distributed, stateful agents.

![Architecture Diagram](architecture.png)

*Figure 1: Golem Social Net Application Architecture*

## Architecture Components

### Agents
- **User Agent**: Manages the user and user connection related operations.
- **Post Agent**: Handles post and comments.
- **User Post Agent**: Manages list of user posts
- **User Timeline Agent**: Manages user timeline.

### Key Features
- **RESTful API** for all shopping operations
- **Stateful Agents** with Golem Cloud managing the state
- **AI Integration** with external LLM service for the shopping assistant
- **Distributed Agent System** with clear responsibility boundaries

### Communication Flow
TODO

### State Management
All core agents (User, User Posts, User Timeline, Post) have their state managed by Golem Cloud, ensuring reliability and scalability through the agent-based architecture.


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

