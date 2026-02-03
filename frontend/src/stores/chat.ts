import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { api, type Chat, type ChatRef, type LikeType } from '../api';
import { useUserStore } from './user';

export const useChatStore = defineStore('chat', () => {
    const userStore = useUserStore();
    const chats = ref<Chat[]>([]);
    const activeChatId = ref<string | null>(null);
    const isLoading = ref(false);
    const error = ref<string | null>(null);
    const lastUpdate = ref<string>(new Date(0).toISOString());
    const isPolling = ref(false);

    const activeChat = computed(() =>
        chats.value.find(c => c['chat-id'] === activeChatId.value) || null
    );

    async function fetchChats() {
        if (!userStore.userId) return;
        isLoading.value = true;
        try {
            const response = await api.getChats(userStore.userId);
            if (response.data.ok) {
                chats.value = response.data.ok;
                // Update lastUpdate based on the most recent chat update
                const latest = chats.value.reduce((latest, chat) => {
                    const chatTime = new Date(chat['updated-at'].timestamp).getTime();
                    return chatTime > latest ? chatTime : latest;
                }, 0);
                if (latest > 0) {
                    lastUpdate.value = new Date(latest).toISOString();
                }
            }
        } catch (e: any) {
            error.value = e.message;
        } finally {
            isLoading.value = false;
        }
    }

    async function pollUpdates() {
        if (!userStore.userId || !isPolling.value) return;
        try {
            const response = await api.getChatUpdates(userStore.userId, lastUpdate.value);
            if (response.data.ok) {
                const updates: ChatRef[] = response.data.ok;
                if (updates.length > 0) {
                    // If there are updates, refetch all chats to get full content
                    // In a real app we'd fetch only changed chats if API allowed it
                    await fetchChats();
                }
            }
        } catch (e) {
            // Silently fail polling or log it
            console.error('Polling error:', e);
        } finally {
            if (isPolling.value) {
                setTimeout(pollUpdates, 5000); // Poll every 5 seconds
            }
        }
    }

    function startPolling() {
        if (isPolling.value) return;
        isPolling.value = true;
        pollUpdates();
    }

    function stopPolling() {
        isPolling.value = false;
    }

    async function sendMessage(content: string) {
        if (!activeChatId.value || !userStore.userId) return;
        try {
            await api.addChatMessage(activeChatId.value, userStore.userId, content);
            await fetchChats(); // Refresh to get the new message
        } catch (e: any) {
            error.value = e.message;
        }
    }

    async function toggleLike(messageId: string, likeType: LikeType = 'like') {
        if (!activeChatId.value || !userStore.userId) return;
        const chat = activeChat.value;
        if (!chat) return;
        const message = chat.messages.find(m => m['message-id'] === messageId);
        if (!message) return;

        const existingLike = message.likes?.find(([uid]) => uid === userStore.userId);
        const hasLiked = existingLike ? existingLike[1] : null;

        try {
            if (hasLiked === likeType) {
                await api.unlikeChatMessage(activeChatId.value, messageId, userStore.userId);
            } else {
                await api.likeChatMessage(activeChatId.value, messageId, userStore.userId, likeType);
            }
            await fetchChats();
        } catch (e: any) {
            error.value = e.message;
        }
    }

    async function createNewChat(participants: string[]) {
        if (!userStore.userId) return;
        isLoading.value = true;
        try {
            const response = await api.createChat(userStore.userId, participants);
            if (response.data.ok) {
                const chatId = response.data.ok['chat-id'];
                await fetchChats();
                activeChatId.value = chatId;
                return chatId;
            }
        } catch (e: any) {
            error.value = e.message;
            throw e;
        } finally {
            isLoading.value = false;
        }
    }

    async function addParticipant(chatId: string, participantIds: string[]) {
        isLoading.value = true;
        try {
            const response = await api.addChatParticipant(chatId, participantIds);
            if (response.data.ok) {
                await fetchChats();
            }
        } catch (e: any) {
            error.value = e.message;
            throw e;
        } finally {
            isLoading.value = false;
        }
    }

    async function goToChatWithUser(targetUserId: string) {
        if (!userStore.userId) return;

        // Find existing chat with ONLY this participant
        const existing = chats.value.find(c =>
            c.participants.length === 2 &&
            c.participants.includes(targetUserId)
        );

        if (existing) {
            activeChatId.value = existing['chat-id'];
        } else {
            // Create new one
            await createNewChat([targetUserId]);
        }
    }

    return {
        chats,
        activeChatId,
        activeChat,
        isLoading,
        error,
        fetchChats,
        startPolling,
        stopPolling,
        sendMessage,
        toggleLike,
        createNewChat,
        addParticipant,
        goToChatWithUser
    };
});
