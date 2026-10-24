import axios from 'axios';

export const API_BASE_URL = '/api/v1/social-net';

export const apiClient = axios.create({
    baseURL: API_BASE_URL,
    headers: {
        'Content-Type': 'application/json',
    },
});

export type UserConnectionType = 'Friend' | 'Following' | 'Follower'

export interface Timestamp {
    timestamp: string;
}

// Types based on inferred backend usage

export interface ConnectedUser {
    user_id: string;
    connection_types: UserConnectionType[];
    created_at: Timestamp;
    updated_at: Timestamp;
}
// Connected users is a list of tuples: [userId, UserDetails]
export type ConnectedUserTuple = [string, ConnectedUser];

export interface User {
    user_id: string;
    name?: string;
    email?: string;
    created_at?: Timestamp; // Enforced Timestamp only
    connected_users?: ConnectedUserTuple[];
}

export type LikeType = 'Like' | 'Insightful' | 'Love' | 'Dislike';

export type UserLikeTuple = [string, LikeType];

export interface Comment {
    comment_id: string;
    parent_comment_id?: string;
    content: string;
    likes?: UserLikeTuple[];
    created_by: string;
    created_at: Timestamp;
}
// Comments is a list of tuples: [commentId, Comment]
export type CommentTuple = [string, Comment];

export interface Post {
    post_id: string;
    content: string;
    created_by: string;
    created_at: Timestamp;
    likes?: UserLikeTuple[];
    comments?: CommentTuple[];
}

export interface PostRef {
    post_id: string;
    created_by: string;
    created_by_connection_type?: UserConnectionType;
    created_at: Timestamp;
}

export interface TimelineUpdates {
    user_id: string;
    posts: PostRef[];
}

export interface ConnectionRequest {
    user_id: string; // The target user ID
    connection_type: UserConnectionType; // Assuming these types
}

export const convertToSnakeCase = (obj: any) => {
    // Helper if we need to convert camelCase to snake_case for backend
    // But currently backend seems to expect json body, fields like `user_id`.
    return obj;
}

export const api = {
    getUser: (userId: string) => apiClient.get(`/users/${userId}`),
    updateName: (userId: string, name: string) => apiClient.put(`/users/${userId}/name`, { name }),
    updateEmail: (userId: string, email: string) => apiClient.put(`/users/${userId}/email`, { email }),

    createPost: (userId: string, content: string) => apiClient.post(`/users/${userId}/posts`, { content }),
    getPosts: (userId: string, query: string = '') => apiClient.get(`/users/${userId}/posts/search`, { params: { query } }),

    getTimeline: (userId: string, query: string = '') => apiClient.get(`/users/${userId}/timeline/posts`, { params: { query } }),

    getTimelineUpdates: (userId: string, since: string) => apiClient.get(`/users/${userId}/timeline/posts/updates`, { params: { since } }),

    searchUsers: (query: string) => apiClient.get(`/users/search`, { params: { query } }),

    connectUser: (userId: string, targetUserId: string, type: UserConnectionType = 'Following') =>
        apiClient.put(`/users/${userId}/connections`, { user_id: targetUserId, connection_type: type }),

    disconnectUser: (userId: string, targetUserId: string, type: UserConnectionType = 'Following') =>
        apiClient.request({
            method: 'DELETE',
            url: `/users/${userId}/connections`,
            data: { user_id: targetUserId, connection_type: type }
        }),

    addComment: (postId: string, userId: string, content: string, parentCommentId?: string) =>
        apiClient.post(`/posts/${postId}/comments`, { user_id: userId, content, parent_comment_id: parentCommentId }),

    deleteComment: (postId: string, commentId: string) =>
        apiClient.delete(`/posts/${postId}/comments/${commentId}`),

    likePost: (postId: string, userId: string, likeType: LikeType) =>
        apiClient.put(`/posts/${postId}/likes`, { user_id: userId, like_type: likeType }),

    unlikePost: (postId: string, userId: string) =>
        apiClient.delete(`/posts/${postId}/likes/${userId}`),

    likeComment: (postId: string, commentId: string, userId: string, likeType: LikeType) =>
        apiClient.put(`/posts/${postId}/comments/${commentId}/likes`, { user_id: userId, like_type: likeType }),

    unlikeComment: (postId: string, commentId: string, userId: string) =>
        apiClient.delete(`/posts/${postId}/comments/${commentId}/likes/${userId}`),

    // Chat APIs
    createChat: (userId: string, participants: string[]) =>
        apiClient.post(`/users/${userId}/chats`, { participants }),

    getChats: (userId: string, query: string = '') =>
        apiClient.get(`/users/${userId}/chats/search`, { params: { query } }),

    getChatUpdates: (userId: string, since: string) =>
        apiClient.get(`/users/${userId}/chats/updates`, { params: { since } }),

    addChatMessage: (chatId: string, userId: string, content: string) =>
        apiClient.post(`/chats/${chatId}/messages`, { user_id: userId, content }),

    deleteChatMessage: (chatId: string, messageId: string) =>
        apiClient.delete(`/chats/${chatId}/messages/${messageId}`),

    likeChatMessage: (chatId: string, messageId: string, userId: string, likeType: LikeType) =>
        apiClient.put(`/chats/${chatId}/messages/${messageId}/likes`, { user_id: userId, like_type: likeType }),

    unlikeChatMessage: (chatId: string, messageId: string, userId: string) =>
        apiClient.delete(`/chats/${chatId}/messages/${messageId}/likes/${userId}`),

    addChatParticipant: (chatId: string, participants: string[]) =>
        apiClient.patch(`/chats/${chatId}/participants`, { participants }),
};

export interface Message {
    message_id: string;
    content: string;
    likes: UserLikeTuple[];
    created_by: string;
    created_at: Timestamp;
    updated_at: Timestamp;
}

export interface Chat {
    chat_id: string;
    created_by: string;
    participants: string[];
    messages: Message[];
    created_at: Timestamp;
    updated_at: Timestamp;
}

export interface ChatRef {
    chat_id: string;
    created_at: Timestamp | string;
    updated_at: Timestamp | string;
    created_by?: string;
}

export interface UserChats {
    user_id: string;
    chats: ChatRef[];
    created_at: Timestamp;
    updated_at: Timestamp;
}

export interface UserChatsUpdates {
    user_id: string;
    chats: ChatRef[];
}

/**
 * Stream chat updates from the backend using Golem Durable Streams protocol (PUT + SSE).
 */
export async function streamChatUpdates(
    userId: string,
    since: string,
    signal: AbortSignal,
    onUpdate: (update: ChatRef) => void
): Promise<void> {
    if (signal.aborted) return;

    const params = new URLSearchParams();
    if (since) {
        params.set('since', since);
    }
    params.set('iter-wait-time', '1000');

    // 1. PUT to create or ensure the invocation session
    const putResponse = await fetch(`${API_BASE_URL}/users/${userId}/chats/updates/stream?${params.toString()}`, {
        method: 'PUT',
        signal,
        headers: {
            'Content-Type': 'application/json',
            'Stream-TTL': '300', // 5 minutes sliding idle timeout
        },
    });

    if (!putResponse.ok) {
        throw new Error(`Failed to create chat stream session: ${putResponse.status} ${putResponse.statusText}`);
    }

    const location = putResponse.headers.get('Location');
    if (!location) {
        throw new Error('Missing Location header from stream session creation');
    }

    // Resolve session path ensuring Vite /api proxy prefix
    let sessionPath = location;
    try {
        if (sessionPath.startsWith('http://') || sessionPath.startsWith('https://')) {
            sessionPath = new URL(sessionPath).pathname;
        }
    } catch {}
    if (!sessionPath.startsWith('/api')) {
        sessionPath = `/api${sessionPath.startsWith('/') ? '' : '/'}${sessionPath}`;
    }

    const slotUrl = `${sessionPath}/streams/%24result?live=sse&offset=-1`;

    // 2. Open EventSource on the slot
    return new Promise((resolve, reject) => {
        if (signal.aborted) {
            fetch(sessionPath, { method: 'DELETE' }).catch(() => {});
            resolve();
            return;
        }

        const eventSource = new EventSource(slotUrl);
        let isClosed = false;

        const cleanup = () => {
            if (isClosed) return;
            isClosed = true;
            eventSource.close();
            // 3. DELETE session to immediately cancel the stream on the agent and free worker resources
            fetch(sessionPath, { method: 'DELETE' }).catch(() => {});
        };

        signal.addEventListener('abort', () => {
            cleanup();
            resolve();
        });

        const handleData = (event: MessageEvent) => {
            try {
                const data = JSON.parse(event.data);
                if (Array.isArray(data)) {
                    data.forEach(item => onUpdate(item));
                } else if (data && typeof data === 'object') {
                    onUpdate(data);
                }
            } catch (err) {
                console.warn('Failed to parse SSE chat update:', err);
            }
        };

        // Golem sends 'event: data', while standard SSE may send 'event: message' or default
        eventSource.addEventListener('data', handleData as EventListener);
        eventSource.addEventListener('message', handleData as EventListener);
        eventSource.onmessage = handleData;

        eventSource.addEventListener('control', (event: any) => {
            try {
                const control = JSON.parse(event.data);
                if (control?.streamClosed) {
                    cleanup();
                    resolve();
                }
            } catch {}
        });

        eventSource.onerror = (err) => {
            if (eventSource.readyState === EventSource.CLOSED || !isClosed) {
                cleanup();
                reject(err);
            }
        };
    });
}

