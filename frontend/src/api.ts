import axios from 'axios';

export const API_BASE_URL = '/api/v1/social-net';

export const apiClient = axios.create({
    baseURL: API_BASE_URL,
    headers: {
        'Content-Type': 'application/json',
    },
});

// Types based on inferred backend usage

export interface ConnectedUser {
    'user-id': string;
    'connection-types': string[];
    'created-at': { timestamp: string };
    'updated-at': { timestamp: string };
}
// Connected users is a list of tuples: [userId, UserDetails]
export type ConnectedUserTuple = [string, ConnectedUser];

export interface User {
    'user-id': string;
    name?: string;
    email?: string;
    'created-at'?: { timestamp: string } | string; // Supporting both formats encountered
    'connected-users'?: ConnectedUserTuple[];
}

export interface Comment {
    'comment-id': string;
    'parent-comment-id'?: string;
    content: string;
    'created-by': string;
    'created-at': { timestamp: string } | string;
}
// Comments is a list of tuples: [commentId, Comment]
export type CommentTuple = [string, ConnectedUser];

export interface Post {
    'post-id': string;
    content: string;
    'created-by': string;
    'created-at': { timestamp: string } | string;
    comments?: CommentTuple[];
}

export interface ConnectionRequest {
    'user-id': string; // The target user ID
    'connection-type': 'friend' | 'following'; // Assuming these types
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
    getPosts: (userId: string) => apiClient.get(`/users/${userId}/posts`),

    getTimeline: (userId: string, query: string = '') => apiClient.get(`/users/${userId}/timeline/posts`, { params: { query } }),

    searchUsers: (query: string) => apiClient.get(`/users/search`, { params: { query } }),

    connectUser: (userId: string, targetUserId: string, type: 'friend' | 'following' = 'following') =>
        apiClient.put(`/users/${userId}/connections`, { 'user-id': targetUserId, 'connection-type': type }),

    disconnectUser: (userId: string, targetUserId: string, type: 'friend' | 'following' = 'following') =>
        apiClient.request({
            method: 'DELETE',
            url: `/users/${userId}/connections`,
            data: { 'user-id': targetUserId, 'connection-type': type }
        }),

    addComment: (postId: string, userId: string, content: string, parentCommentId?: string) =>
        apiClient.post(`/posts/${postId}/comments`, { 'user-id': userId, content, 'parent-comment-id': parentCommentId }),
};
