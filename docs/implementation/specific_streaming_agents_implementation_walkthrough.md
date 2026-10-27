# Feature Implementation Walkthrough: Specific Streaming Agents for Timeline and Chat Updates

- **Feature ID / Slug**: `specific-streaming-agents`
- **Date**: 2026-10-27
- **Status**: Approved <!-- Ready for Review | Approved -->

---

## 1. Executive Summary

This feature resolves the issue where streaming endpoints (`get_posts_updates_stream` and `get_chats_updates_stream`) were improperly located on ephemeral workers (`UserTimelineUpdatesAgent` and `UserChatsUpdatesAgent`). Ephemeral workers cannot durably suspend and are unsuitable for Golem's Durable Streams sessions and long-lived background polling loops.

We separated the responsibilities by:
1. Creating dedicated durable agents with `phantom_agent = true`:
   - `UserTimelineUpdatesStreamAgent` in [`src/user_timeline/mod.rs`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/src/user_timeline/mod.rs)
   - `UserChatsUpdatesStreamAgent` in [`src/user_chats/mod.rs`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/src/user_chats/mod.rs)
2. Streamlining method names by removing the redundant `_stream` suffix:
   - `UserTimelineUpdatesStreamAgent::get_posts_updates` returning `AgentStream<PostRef>`
   - `UserChatsUpdatesStreamAgent::get_chats_updates` returning `AgentStream<ChatRef>`
3. Preserving the exact HTTP paths for full API and frontend compatibility:
   - `GET /v1/social-net/users/{user_id}/timeline/posts/updates/stream`
   - `GET /v1/social-net/users/{user_id}/chats/updates/stream`
4. Registering both agents in [`golem.yaml`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/golem.yaml).
5. Updating architecture documentation in [`README.md`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/README.md).

All automated checks passed: formatting (`cargo fmt --check`), compilation (`cargo check --target wasm32-wasip2`), linter (`cargo clippy --target wasm32-wasip2`), full component build (`golem build --yes`), and all 91 unit tests (`cargo test`).

---

## 2. Changes Implemented

### File Modifications

| Action     | File Path                                          | Summary of Changes     |
| :--------- | :------------------------------------------------- | :--------------------- |
| `[MODIFY]` | [`src/common/mod.rs`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/src/common/mod.rs) | Replaced `wstd::task::sleep` with `thread::sleep` in `stream_updates` to avoid runtime panic outside of `wstd` reactor. |
| `[MODIFY]` | [`src/user_timeline/mod.rs`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/src/user_timeline/mod.rs) | Removed `get_posts_updates_stream` from ephemeral `UserTimelineUpdatesAgent`. Defined and implemented `UserTimelineUpdatesStreamAgent` (`phantom_agent = true`) with method `get_posts_updates`. |
| `[MODIFY]` | [`src/user_chats/mod.rs`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/src/user_chats/mod.rs) | Removed `get_chats_updates_stream` from ephemeral `UserChatsUpdatesAgent`. Defined and implemented `UserChatsUpdatesStreamAgent` (`phantom_agent = true`) with method `get_chats_updates`. |
| `[MODIFY]` | [`golem.yaml`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/golem.yaml) | Added `UserChatsUpdatesStreamAgent` and `UserTimelineUpdatesStreamAgent` under `httpApi.deployments.local.agents`. |
| `[MODIFY]` | [`README.md`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/README.md) | Added Streaming Agents section and updated Real-time Synchronization documentation. |
| `[MODIFY]` | [`frontend/src/api.ts`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/frontend/src/api.ts) | Added `streamTimelineUpdates` and extracted reusable `streamUpdates<T>` helper for Durable Streams SSE connections. |
| `[MODIFY]` | [`frontend/src/views/HomeView.vue`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/frontend/src/views/HomeView.vue) | Switched live timeline updates from HTTP long-polling to `streamTimelineUpdates` (SSE). |
| `[MODIFY]` | [`docs/implementation/specific_streaming_agents_feature_plan.md`](file:///Users/coon/workspace-zv/git/golem-social-net-rust/docs/implementation/specific_streaming_agents_feature_plan.md) | Created and maintained feature implementation plan with user approvals. |

### Key Logic & API Changes

- **UserTimelineUpdatesStreamAgent**:
  ```rust
  #[agent_definition(mount = "/v1/social-net/users", phantom_agent = true, cors = ["*"])]
  trait UserTimelineUpdatesStreamAgent {
      fn new() -> Self;

      #[endpoint(
          get = "/{user_id}/timeline/posts/updates/stream?since={since}&iter-wait-time={iter_wait_time}"
      )]
      fn get_posts_updates(
          &mut self,
          user_id: String,
          since: Option<String>,
          iter_wait_time: Option<u32>,
      ) -> golem_rust::agentic::AgentStream<PostRef>;
  }
  ```
- **UserChatsUpdatesStreamAgent**:
  ```rust
  #[agent_definition(mount = "/v1/social-net/users", phantom_agent = true, cors = ["*"])]
  trait UserChatsUpdatesStreamAgent {
      fn new() -> Self;

      #[endpoint(
          get = "/{user_id}/chats/updates/stream?since={since}&iter-wait-time={iter_wait_time}"
      )]
      fn get_chats_updates(
          &mut self,
          user_id: String,
          since: Option<String>,
          iter_wait_time: Option<u32>,
      ) -> golem_rust::agentic::AgentStream<ChatRef>;
  }
  ```
- **Stream Task Sleep Fix**:
  - In `src/common/mod.rs` inside `stream_updates`, replaced `wstd::task::sleep(iter_duration.into()).await;` with `std::thread::sleep(iter_duration);`.
  - `wstd::task::sleep` panicked with `Reactor::current must be called within a wstd runtime` because Golem runs its own async support rather than `wstd`'s reactor.
  - Using `std::thread::sleep` uses native WASI p2 clocks (`monotonic-clock`), preventing panics and keeping the stream alive indefinitely across polling intervals.


---

## 3. Verification & Validation Results

### 3.1 Code Formatting

Command executed:
```bash
cargo fmt --check
```

**Result**:
```text
Exit code: 0 (clean, all files formatted properly)
```

### 3.2 Compilation & Type Checking

Command executed:
```bash
cargo check --target wasm32-wasip2
```

**Result**:
```text
    Checking social_net_rust_social_net v0.0.1 (/Users/coon/workspace-zv/git/golem-social-net-rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.86s
```

### 3.3 Linter Verification (Clippy)

Command executed:
```bash
cargo clippy --target wasm32-wasip2
```

**Result**:
```text
    Checking social_net_rust_social_net v0.0.1 (/Users/coon/workspace-zv/git/golem-social-net-rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.79s
Exit code: 0 (zero warnings, zero errors)
```

### 3.4 Golem Build

Command executed:
```bash
golem build --yes
```

**Result**:
```text
Selecting components
  Found components: social-net-rust:social-net
  Selected components and layers:
    social-net-rust:social-net: rust, rust[release],
         social-net-rust:social-net, social-net-rust:social-net[local]
Building components
  Building social-net-rust:social-net
    Executing external command 'cargo build --target wasm32-wasip2 --release'
         in directory /Users/coon/workspace-zv/git/golem-social-net-rust
 cargo │    Compiling social_net_rust_social_net v0.0.1 (/Users/coon/workspace-zv/git/golem-social-net-rust)
 cargo │     Finished `release` profile [optimized] target(s) in 28.40s
  Adding metadata to social-net-rust:social-net

Finished building [OK]
```

### 3.5 Automated Unit & Integration Tests

Command executed:
```bash
cargo test
```

**Result**:
```text
running 91 tests
test chat::tests::test_chat_matches_query_created_by ... ok
... [all tests passed] ...
test user::tests::test_user_index_shards_distribution ... ok

test result: ok. 91 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

## 4. How to Run & Verify

1. **Build the Application**:
   ```bash
   golem build --yes
   ```
2. **Deploy locally**:
   ```bash
   golem deploy --yes
   ```
3. **Verify Stream Invocations**:
   - HTTP PUT to create a streaming session for chat updates:
     ```bash
     curl -i -X PUT "http://localhost:9006/v1/social-net/users/user-1/chats/updates/stream?iter-wait-time=1000"
     ```
   - Connect via Server-Sent Events (SSE) to the returned `Location` header URL slot:
     ```bash
     curl -N "http://localhost:9006<location>/streams/%24result?live=sse&offset=-1"
     ```
   - Or run the frontend application:
     ```bash
     cd frontend && npm run dev
     ```

---

## 5. Sign-off / Next Steps

- All implementation requirements and skill validation criteria are satisfied.
- Ready for final user approval (Gate 2).
