<script setup lang="ts">
import { ref } from 'vue';
import { api, type User } from '../api';
// I didn't install lodash. I'll use custom debounce.
import { useRouter } from 'vue-router';

// function debounce removed


const query = ref('');
const results = ref<User[]>([]);
const isSearching = ref(false);
const router = useRouter();

const performSearch = async () => {
  const q = query.value.trim();
  if (!q) {
    results.value = [];
    return;
  }
  
  isSearching.value = true;
  try {
    const response = await api.searchUsers(q);
    const data = response.data as any; // Cast to any to access dynamic structure
    if (data && Array.isArray(data.ok)) {
        results.value = data.ok;
    } else {
        results.value = [];
    }
  } catch (err) {
    console.error(err);
    results.value = [];
  } finally {
    isSearching.value = false;
  }
};

function goToProfile(userId: string) {
    router.push(`/profile/${userId}`);
}
</script>

<template>
  <div class="max-w-2xl mx-auto">
    <div class="mb-8">
       <h1 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-400 to-purple-600 mb-2">
         Search
       </h1>
       <p class="text-gray-400">Find people to follow</p>
    </div>

    <div class="mb-8 relative flex gap-2">
      <input 
        v-model="query"
        type="text"
        placeholder="Search for users..."
        class="flex-1 px-5 py-3 rounded-xl bg-neutral-900 border border-neutral-800 text-white focus:outline-none focus:ring-2 focus:ring-purple-500 transition shadow-lg placeholder-gray-600"
        autofocus
        @keyup.enter="performSearch"
      />
      <button 
        @click="performSearch"
        class="px-6 py-3 rounded-xl bg-purple-600 hover:bg-purple-700 text-white font-bold transition shadow-lg disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="isSearching || !query.trim()"
      >
        Search
      </button>
    </div>

    <div v-if="isSearching" class="flex justify-center py-8">
       <div class="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-purple-500"></div>
    </div>

    <div v-else-if="results.length > 0" class="space-y-4">
      <div 
        v-for="user in results" 
        :key="user['user-id']"
        @click="goToProfile(user['user-id'])"
        class="flex items-center p-4 bg-neutral-900 border border-neutral-800 rounded-xl hover:bg-neutral-800 transition cursor-pointer group"
      >
        <div class="w-12 h-12 rounded-full bg-neutral-800 flex items-center justify-center text-xl font-bold text-gray-300 mr-4 border-2 border-neutral-700 group-hover:border-purple-500 transition">
          {{ user['user-id'].charAt(0).toUpperCase() }}
        </div>
        <div>
           <h3 class="font-bold text-white group-hover:text-purple-400 transition">{{ user.name || user['user-id'] }}</h3>
           <p class="text-sm text-gray-500">@{{ user['user-id'] }}</p>
        </div>
      </div>
    </div>

    <div v-else-if="query && !isSearching" class="text-center py-8 text-gray-500">
       No users found.
    </div>
  </div>
</template>
