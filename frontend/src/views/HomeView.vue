<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { api, type Post } from '../api';
import { useUserStore } from '../stores/user';
import CreatePost from '../components/CreatePost.vue';
import PostCard from '../components/PostCard.vue';

const posts = ref<Post[]>([]);
const isLoading = ref(true);
const userStore = useUserStore();

async function fetchTimeline() {
  if (!userStore.userId) return;
  
  isLoading.value = true;
  try {
    const response = await api.getTimeline(userStore.userId);
    // user-timeline-view-agent().get-posts-view(...) returns Option<Vec<Post>>
    // Backend API mapping: some(x) => ok(x), none => 404
    // If we get 200, it's the array.
    
    // Note: The backend returns plain array in body if found.
    // Axios response.data should be the array.
    const data = response.data as any;
    if (data && Array.isArray(data.ok)) {
        posts.value = data.ok;
    } else {
        posts.value = [];
    }
  } catch (err) {
    console.error('Failed to fetch timeline:', err);
    posts.value = [];
  } finally {
    isLoading.value = false;
  }
}

onMounted(() => {
  fetchTimeline();
});
</script>

<template>
  <div class="max-w-2xl mx-auto">
    <div class="mb-8">
      <h1 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-400 to-purple-600 mb-2">
        Timeline
      </h1>
      <p class="text-gray-400">See what's happening in your network</p>
    </div>

    <CreatePost @post-created="fetchTimeline" />

    <div v-if="isLoading" class="flex justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-purple-500"></div>
    </div>

    <div v-else-if="posts.length === 0" class="text-center py-12 bg-neutral-900 rounded-xl border border-neutral-800">
      <p class="text-gray-400">No posts yet. Start following someone or create a post!</p>
    </div>

    <div v-else class="space-y-6">
      <PostCard 
        v-for="post in posts" 
        :key="post['post-id']" 
        :post="post" 
      />
    </div>
  </div>
</template>
