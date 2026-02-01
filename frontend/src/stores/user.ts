import { defineStore } from 'pinia';
import { ref, computed } from 'vue';

export const useUserStore = defineStore('user', () => {
    const userId = ref<string | null>(localStorage.getItem('userId'));
    const liveTimeline = ref(false);

    const isLoggedIn = computed(() => !!userId.value);

    function login(id: string) {
        userId.value = id;
        localStorage.setItem('userId', id);
    }

    function logout() {
        userId.value = null;
        localStorage.removeItem('userId');
    }

    return {
        userId,
        liveTimeline,
        isLoggedIn,
        login,
        logout,
    };
});
