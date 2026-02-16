# Building a Decentralized Social Network with Golem Cloud Agents

In the evolving landscape of cloud computing, the concept of **agents** is reshaping how we think about distributed systems. Instead of monolithic backends or stateless microservices reliant on external databases, we can now build applications where data and logic coexist within autonomous, persistent entities.

In this post, we'll explore **Golem Social Net**, a proof-of-concept social networking application built on **Golem Cloud**. We'll dive into its agentic architecture, communication flows, and how specific agents are implemented in Rust.

## Overview: The Agentic Paradigm

Traditional web architectures often separate the "compute" (stateless servers) from the "storage" (databases). Golem Cloud challenges this by introducing **Stateful Workers** (Agents). An agent in Golem is a durable execution unit that maintains its own state in memory, which persists across restarts and upgrades.

In our social network, we don't have a central "User Database" or "Post Table". Instead:
- Every **User** is an independent agent.
- Every **Post** is an independent agent.
- Every **Chat** is an independent agent.

This granular approach allows for natural scalability and fault isolation. If one user's agent fails (which Golem handles gracefully), it doesn't bring down the entire system.

## Architecture

The system is composed of a constellation of agents, split into two main categories: **Stateful Agents** (persistent entities) and **Ephemeral Agents** (view aggregators and searchers).

![Architecture Diagram](architecture.png)
*Figure 1: High-level Architecture of Golem Social Net*

### Communication Flow

The system manages interactions through a mix of synchronous RPC calls and asynchronous invocations:

1.  **Request Entry**: The **API Gateway** receives REST requests and routes them to the specific agent (e.g., `GET /users/{id}` -> `User Agent {id}`).
2.  **Discovery**: A **User Search Agent** scans the network to discover specific `User Agents` matching a query.
3.  **Fan-out Distribution**: When a user creates a post:
    *   The **User Posts Agent** initializes a new **Post Agent**.
    *   The **Post Agent** asynchronously invokes the **Timelines Updater Agent**. This is a durable, guaranteed operation.
    *   The **Timelines Updater Agent** looks up the author's followers and "fans out" the post reference to their personal **User Timeline Agents**.
4.  **View Aggregation**: To show a timeline, a **User Timeline View Agent** queries the `User Timeline Agent` for a list of post IDs, then fetches the actual content from multiple `Post Agents` in parallel, constructing a complete view.

## Agent Design

The system's logic is distributed across several specialized agents. Here is a complete breakdown of every agent type in the application.

### 1. Core Entity Agents

These agents represent the primary domain entities.

#### User Agent
The **User Agent** is the persistent identity of a user. It stores profile data and manages the list of connections (friends and followers).

```rust
#[agent_definition]
trait UserAgent {
    fn new(id: String) -> Self;
    fn get_user(&self) -> Option<User>;
    fn set_name(&mut self, name: Option<String>) -> Result<(), String>;
    fn connect_user(&mut self, user_id: String, connection_type: UserConnectionType) -> Result<(), String>;
    // ... disconnect, etc.
}
```

#### Post Agent
The **Post Agent** manages the lifecycle of a single post, including its content, likes, and a tree of comments. It is also responsible for triggering updates when its state changes.

```rust
#[agent_definition]
trait PostAgent {
    fn new(id: String) -> Self;
    fn get_post(&self) -> Option<Post>;
    async fn init_post(&mut self, user_id: String, content: String) -> Result<(), String>;
    fn add_comment(&mut self, user_id: String, content: String, parent_comment_id: Option<String>) -> Result<String, String>;
    fn set_like(&mut self, user_id: String, like_type: LikeType) -> Result<(), String>;
}
```

Here we can see how the `Post Agent` proactively notifies the `Timelines Updater Agent` upon creation:

```rust
async fn init_post(&mut self, user_id: String, content: String) -> Result<(), String> {
    // ... setup state ...
    TimelinesUpdaterAgentClient::get(user_id.clone())
        .trigger_post_updated(PostUpdate::from(state), true);
    // ...
}
```

#### Chat Agent
The **Chat Agent** handles a single chat room. It stores the message history and participation list.

```rust
#[agent_definition]
trait ChatAgent {
    fn new(id: String) -> Self;
    fn get_chat(&self) -> Option<Chat>;
    fn add_message(&mut self, user_id: String, content: String) -> Result<String, String>;
    fn add_participants(&mut self, participants_ids: HashSet<String>) -> Result<(), String>;
}
```

When a message is added, the `Chat Agent` iterates through all participants and updates their individual `User Chats Agent` registries, ensuring their chat lists move to the top:

```rust
fn execute_chat_updates(
    chat_id: String,
    participants_ids: HashSet<String>,
    updated_at: chrono::DateTime<chrono::Utc>,
) {
    for p_id in participants_ids {
        UserChatsAgentClient::get(p_id.clone())
            .trigger_chat_updated(chat_id.clone(), updated_at);
    }
}
```

### 2. Collection & Registry Agents

These stateful agents manage collections of references, linking core entities together.

#### User Posts Agent
This agent acts as a registry for all posts created by a specific user. It generates unique IDs for new posts and delegates the actual creation to a fresh `Post Agent`.

```rust
#[agent_definition]
trait UserPostsAgent {
    fn new(id: String) -> Self;
    fn get_posts(&self) -> Option<UserPosts>;
    fn create_post(&mut self, content: String) -> Result<String, String>;
}
```

The `create_post` function demonstrates how distinct agents are orchestrated. It generates an ID, initializes a specific `Post Agent`, and then stores the reference locally:

```rust
fn create_post(&mut self, content: String) -> Result<String, String> {
    self.with_state(|state| {
        let post_id = uuid::Uuid::new_v4().to_string();

        PostAgentClient::get(post_id.clone())
            .trigger_init_post(state.user_id.clone(), content);

        let post_ref = PostRef::new(post_id.clone());
        state.updated_at = post_ref.created_at;
        state.posts.push(post_ref);

        Ok(post_id)
    })
}
```

#### User Timeline Agent
This agent maintains the personal timeline for a user. It stores references (`PostRef`) to posts from friends and followed users. It receives updates via the fan-out mechanism.

```rust
#[agent_definition]
trait UserTimelineAgent {
    fn new(id: String) -> Self;
    fn get_timeline(&self) -> Option<UserTimeline>;
    fn posts_updated(&mut self, posts: Vec<PostRef>) -> Result<(), String>;
}
```

#### User Chats Agent
Similar to `User Posts Agent`, this registry tracks all chat rooms a user is a participant in.

```rust
#[agent_definition]
trait UserChatsAgent {
    fn new(id: String) -> Self;
    fn get_chats(&self) -> Option<UserChats>;
    fn create_chat(&mut self, participants_ids: HashSet<String>) -> Result<String, String>;
    fn add_chat(&mut self, chat_id: String, created_by: String, created_at: chrono::DateTime<chrono::Utc>) -> Result<(), String>;
}
```

Triggering a new chat involves a similar pattern: creating the ID, initializing the dedicated `Chat Agent` with participants, and updating the local list.

```rust
fn create_chat(&mut self, participants_ids: HashSet<String>) -> Result<String, String> {
    self.with_state(|state| {
        // ... validation ...
        let chat_id = uuid::Uuid::new_v4().to_string();

        ChatAgentClient::get(chat_id.clone()).trigger_init_chat(
            participants_ids,
            state.user_id.clone(),
            created_at,
        );

        state.chats.push(ChatRef::new(chat_id.clone()));
        // ...
        Ok(chat_id)
    })
}
```

### 3. Orchestration & Updates Agents

These agents handle background processing and real-time updates.

#### Timelines Updater Agent
This is the "fan-out" worker. When a post is created, this agent determines who needs to see it and pushes the update to their respective `User Timeline Agents`.

```rust
#[agent_definition]
trait TimelinesUpdaterAgent {
    fn new(id: String) -> Self;
    async fn post_updated(&mut self, update: PostUpdate, process_immediately: bool);
    async fn process_posts_updates(&mut self);
}
```

The `post_updated` method receives the update. If `process_immediately` is true, it triggers the fan-out right away. Otherwise, it buffers the update.

```rust
 async fn post_updated(&mut self, update: PostUpdate, process_immediately: bool) {
    self.add_update(update);

    if process_immediately {
        self.execute_posts_updates().await;
    }
}
```

The `execute_posts_updates` (helper function) demonstrates the logic of finding followers and pushing data to them:

```rust
async fn execute_posts_updates(user_id: String, updates: Vec<PostUpdate>) -> bool {
    // 1. Fetch the author's profile to get connections
    let user = UserAgentClient::get(user_id.clone()).get_user().await;

    if let Some(user) = user {
        // 2. Identify followers and friends
        let mut notify_user_ids: HashMap<String, UserConnectionType> = HashMap::new();
        // ... filter connected_users ...

        // 3. Push updates to each follower's timeline
        for (connected_user_id, connection_type) in notify_user_ids {
            let user_updates = updates.clone().into_iter().map(|update| {
                 // ... create PostRef ...
            }).collect();
            
            UserTimelineAgentClient::get(connected_user_id)
                .trigger_posts_updated(user_updates);
        }
        true
    } else {
        false
    }
}
```

#### User Timeline Updates Agent (Ephemeral)
This ephemeral agent implements a **long-polling** mechanism. It checks the `User Timeline Agent` for any changes since a specific timestamp, allowing the frontend to receive real-time updates without constant refreshing.

```rust
#[agent_definition(mode = "ephemeral")]
trait UserTimelineUpdatesAgent {
    fn new() -> Self;
    async fn get_posts_updates(
        &mut self,
        user_id: String,
        updates_since: Option<chrono::DateTime<chrono::Utc>>,
        iter_wait_time: Option<u32>,
        max_wait_time: Option<u32>,
    ) -> Option<Vec<PostRef>>;
}
```

The implementation leverages a generic `poll_for_updates` helper to handle the loop and timeout logic, keeping the agent code clean and focused on the specific data retrieval:

```rust
async fn get_posts_updates(
    &mut self,
    user_id: String,
    updates_since: Option<chrono::DateTime<chrono::Utc>>,
    iter_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
) -> Option<Vec<PostRef>> {
    poll_for_updates(
        user_id,
        updates_since,
        iter_wait_time,
        max_wait_time,
        |uid, since| async move {
            let res = UserTimelineAgentClient::get(uid).get_updates(since).await;
            res.map(|r| r.posts)
        },
        "get posts updates",
    )
    .await
}
```

Here is the generic `poll_for_updates` function that encapsulates the polling logic:

```rust
pub async fn poll_for_updates<T, F, Fut>(
    user_id: String,
    updates_since: Option<chrono::DateTime<chrono::Utc>>,
    iter_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    get_updates_fn: F,
    log_prefix: &str,
) -> Option<T>
where
    F: Fn(String, chrono::DateTime<chrono::Utc>) -> Fut,
    Fut: std::future::Future<Output = Option<T>>,
{
    let since = updates_since.unwrap_or(chrono::Utc::now());
    let max_wait_time = Duration::from_millis(max_wait_time.unwrap_or(10000) as u64);
    let iter_wait_time = Duration::from_millis(iter_wait_time.unwrap_or(1000) as u64);
    let now = Instant::now();
    let mut done = false;
    let mut result: Option<T> = None;

    while !done {
        let res = get_updates_fn(user_id.clone(), since).await;

        if let Some(updates) = res {
            result = Some(updates);
            done = true;
        } else {
            done = now.elapsed() >= max_wait_time;
            if !done {
                thread::sleep(iter_wait_time);
            }
        }
    }
    result
}
```

#### User Chats Updates Agent (Ephemeral)
Similar to the timeline updater, this agent provides long-polling for the user's chat list, ensuring they see new conversations immediately.

```rust
#[agent_definition(mode = "ephemeral")]
trait UserChatsUpdatesAgent {
    fn new() -> Self;
    async fn get_chats_updates(
        &mut self,
        user_id: String,
        updates_since: Option<chrono::DateTime<chrono::Utc>>,
        iter_wait_time: Option<u32>,
        max_wait_time: Option<u32>,
    ) -> Option<Vec<ChatRef>>;
}
```

### 4. View & Discovery Agents (Ephemeral)

These agents are stateless aggregators. They query multiple stateful agents in parallel to build complete views for the frontend.

#### User Search Agent
This agent leverages Golem's platform capabilities to scan the network for `User Agents` that match a specific name or criteria.

```rust
#[agent_definition(mode = "ephemeral")]
trait UserSearchAgent {
    fn new() -> Self;
    async fn search(&self, query: String) -> Result<Vec<User>, String>;
}
```

#### User Posts View Agent
It aggregates data by getting a list of post IDs from `User Posts Agent` and then fetching full content from each `Post Agent` in parallel.

```rust
#[agent_definition(mode = "ephemeral")]
trait UserPostsViewAgent {
    fn new() -> Self;
    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>>;
}
```

This agent illustrates the power of the "scatter-gather" pattern in Golem. It initially retrieves the complete list of post IDs from the `User Posts Agent` and subsequently launches parallel requests to fetch the actual post data:

```rust
async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>> {
    let user_posts = UserPostsAgentClient::get(user_id.clone()).get_posts().await;

    if let Some(user_posts) = user_posts {
        let query_matcher = PostQueryMatcher::new(&query);
        let user_posts = user_posts.posts;
        
        if user_posts.is_empty() {
            Some(vec![])
        } else {
            let mut result: Vec<Post> = vec![];

            for chunk in user_posts.chunks(10) {
                let clients = chunk
                    .iter()
                    .map(|p| PostAgentClient::get(p.post_id.clone()))
                    .collect::<Vec<_>>();

                let tasks: Vec<_> = clients.iter().map(|client| client.get_post()).collect();

                let responses = join_all(tasks).await;

                let chunk_result: Vec<Post> = responses
                    .into_iter()
                    .flatten()
                    .filter(|p| query_matcher.matches_post(p.clone()))
                    .collect();

                result.extend(chunk_result);
            }

            Some(result)
        }
    } else {
        None
    }
}
```

#### User Timeline View Agent
This agent is similar to the **User Posts View Agent**, but it aggregates the user's timeline.

```rust
#[agent_definition(mode = "ephemeral")]
trait UserTimelineViewAgent {
    fn new() -> Self;
    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>>;
}
```

#### User Chats View Agent
This agent aggregates full chat states for the user's chat list.

```rust
#[agent_definition(mode = "ephemeral")]
trait UserChatsViewAgent {
    fn new() -> Self;
    async fn get_chats_view(&mut self, user_id: String, query: String) -> Option<Vec<Chat>>;
}
```

## Frontend

While the backend infrastructure is a network of agents, the frontend is a web application.

Built with **Vue 3**, **TypeScript**, and **Tailwind CSS**, it communicates with the Golem agents via a standard REST API exposed by the Golem Gateway. The frontend doesn't need to know it's talking to thousands of distributed agents; it just makes HTTP requests like any other SPA. All the complexity of routing to the correct agent is handled by the Golem infrastructure.

---

**Conclusion**

Golem Social Net demonstrates that complex, stateful applications can be built without managing databases or monolithic application servers. By treating every entity as an autonomous agent, we gain a system that is naturally modular, scalable, and resilient.


## Next Steps

1. Explore the [GitHub repository](https://github.com/justcoon/golem-social-net-rust)
2. Try deploying your own instance
3. Contribute to the project

## Resources

- [Golem Documentation](https://learn.golem.cloud/)
- [Rust Programming Language](https://www.rust-lang.org/)
- [WebAssembly](https://webassembly.org/)
