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
    created_at: Timestamp;
    updated_at: Timestamp;
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
