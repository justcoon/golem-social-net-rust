import axios from 'axios';

export const API_BASE_URL = '/api/v1/social-net';

export const apiClient = axios.create({
    baseURL: API_BASE_URL,
    headers: {
        'Content-Type': 'application/json',
    },
});

export type UserConnectionType = 'friend' | 'following' | 'follower'

export interface Timestamp {
    timestamp: string;
}

// Types based on inferred backend usage

export interface ConnectedUser {
    'user-id': string;
    'connection-types': UserConnectionType[];
    'created-at': Timestamp;
    'updated-at': Timestamp;
}
// Connected users is a list of tuples: [userId, UserDetails]
export type ConnectedUserTuple = [string, ConnectedUser];

export interface User {
    'user-id': string;
    name?: string;
    email?: string;
    'created-at'?: Timestamp; // Enforced Timestamp only
    'connected-users'?: ConnectedUserTuple[];
}

export type LikeType = 'like' | 'insightful' | 'love' | 'dislike';

export type UserLikeTuple = [string, LikeType];

export interface Comment {
    'comment-id': string;
    'parent-comment-id'?: string;
    content: string;
    likes?: UserLikeTuple[];
    'created-by': string;
    'created-at': Timestamp;
}
// Comments is a list of tuples: [commentId, Comment]
export type CommentTuple = [string, Comment];

export interface Post {
    'post-id': string;
    content: string;
    'created-by': string;
    'created-at': Timestamp;
    likes?: UserLikeTuple[];
    comments?: CommentTuple[];
}

export interface PostRef {
    'post-id': string;
    'created-by': string;
    'created-by-connection-type'?: UserConnectionType;
    'created-at': Timestamp;
}

export interface TimelineUpdates {
    'user-id': string;
    posts: PostRef[];
}

export interface ConnectionRequest {
    'user-id': string; // The target user ID
    'connection-type': UserConnectionType; // Assuming these types
}

export const convertToKebabCase = (obj: any) => {
    // Helper if we need to convert camelCase to kebab-case for backend
    // But currently backend seems to expect json body, fields like `user-id`.
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

    connectUser: (userId: string, targetUserId: string, type: UserConnectionType = 'following') =>
        apiClient.put(`/users/${userId}/connections`, { 'user-id': targetUserId, 'connection-type': type }),

    disconnectUser: (userId: string, targetUserId: string, type: UserConnectionType = 'following') =>
        apiClient.request({
            method: 'DELETE',
            url: `/users/${userId}/connections`,
            data: { 'user-id': targetUserId, 'connection-type': type }
        }),

    addComment: (postId: string, userId: string, content: string, parentCommentId?: string) =>
        apiClient.post(`/posts/${postId}/comments`, { 'user-id': userId, content, 'parent-comment-id': parentCommentId }),

    deleteComment: (postId: string, commentId: string) =>
        apiClient.delete(`/posts/${postId}/comments/${commentId}`),

    likePost: (postId: string, userId: string, likeType: LikeType) =>
        apiClient.put(`/posts/${postId}/likes`, { 'user-id': userId, 'like-type': likeType }),

    unlikePost: (postId: string, userId: string) =>
        apiClient.delete(`/posts/${postId}/likes/${userId}`),

    likeComment: (postId: string, commentId: string, userId: string, likeType: LikeType) =>
        apiClient.put(`/posts/${postId}/comments/${commentId}/likes`, { 'user-id': userId, 'like-type': likeType }),

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
        apiClient.post(`/chats/${chatId}/messages`, { 'user-id': userId, content }),

    deleteChatMessage: (chatId: string, messageId: string) =>
        apiClient.delete(`/chats/${chatId}/messages/${messageId}`),

    likeChatMessage: (chatId: string, messageId: string, userId: string, likeType: LikeType) =>
        apiClient.put(`/chats/${chatId}/messages/${messageId}/likes`, { 'user-id': userId, 'like-type': likeType }),

    unlikeChatMessage: (chatId: string, messageId: string, userId: string) =>
        apiClient.delete(`/chats/${chatId}/messages/${messageId}/likes/${userId}`),

    addChatParticipant: (chatId: string, participants: string[]) =>
        apiClient.patch(`/chats/${chatId}/participants`, { participants }),
};

export interface Message {
    'message-id': string;
    content: string;
    likes: UserLikeTuple[];
    'created-by': string;
    'created-at': Timestamp;
    'updated-at': Timestamp;
}

export interface Chat {
    'chat-id': string;
    'created-by': string;
    participants: string[];
    messages: Message[];
    'created-at': Timestamp;
    'updated-at': Timestamp;
}

export interface ChatRef {
    'chat-id': string;
    'created-at': Timestamp;
    'updated-at': Timestamp;
}

export interface UserChats {
    'user-id': string;
    chats: ChatRef[];
    'created-at': Timestamp;
    'updated-at': Timestamp;
}

export interface UserChatsUpdates {
    'user-id': string;
    chats: ChatRef[];
}
