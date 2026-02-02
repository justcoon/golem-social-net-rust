<script setup lang="ts">
import { ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { api, type User, type Post } from '../api';
import { useUserStore } from '../stores/user';
import PostCard from '../components/PostCard.vue';
import CreatePost from '../components/CreatePost.vue';

const route = useRoute();
const router = useRouter();
const userStore = useUserStore();

const user = ref<User | null>(null);
const posts = ref<Post[]>([]);
const isLoading = ref(true);
const error = ref('');

const isCurrentUser = ref(false);

async function loadProfile() {
  const targetId = (route.params.id as string) || userStore.userId;
  if (!targetId) return;

  isCurrentUser.value = targetId === userStore.userId;
  isLoading.value = true;
  error.value = '';
  user.value = null;
  posts.value = [];

  try {
    // Parallel fetch
    // We use getTimeline with a filter to get the user's OWN posts with content.
    // The raw `getPosts` endpoint only returns IDs without content.
    const [userRes, postsRes] = await Promise.allSettled([
      api.getUser(targetId),
      api.getPosts(targetId)
    ]);

    if (userRes.status === 'fulfilled') {
        const data = userRes.value.data as any;
        if (data && data.ok) {
            user.value = data.ok;
        } else {
             // Fallback or error if not in ok format
             // The backend sends 404 if none, so here it might be just ok(x) result
             user.value = data.ok || data;
        }
    } else {
        error.value = 'User not found';
    }

    if (postsRes.status === 'fulfilled') {
        const data = postsRes.value.data as any;
        if (Array.isArray(data.ok)) {
           posts.value = data.ok;
        } else {
           posts.value = [];
        }
    }
  } catch (err) {
    console.error(err);
    error.value = 'Failed to load profile';
  } finally {
    isLoading.value = false;
  }
}

watch(() => route.params.id, () => {
  loadProfile();
}, { immediate: true });

</script>

<template>
  <div class="max-w-4xl mx-auto">
    <div v-if="isLoading" class="flex justify-center py-20">
      <div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-purple-500"></div>
    </div>

    <div v-else-if="error" class="text-center py-20">
      <h2 class="text-2xl font-bold text-red-500">{{ error }}</h2>
    </div>

    <div v-else-if="user" class="space-y-8">
      <!-- Profile Header -->
      <div class="bg-neutral-900 border border-neutral-800 rounded-2xl p-8 relative overflow-hidden">
        <div class="absolute top-0 left-0 w-full h-2 bg-gradient-to-r from-blue-500 to-purple-600"></div>
        
        <div class="flex flex-col md:flex-row items-center md:items-start gap-6">
          <div class="w-24 h-24 rounded-full bg-neutral-800 flex items-center justify-center text-3xl font-bold text-gray-300 border-4 border-neutral-900 shadow-xl">
             {{ user['user-id'].charAt(0).toUpperCase() }}
          </div>
          
          <div class="flex-1 text-center md:text-left">
            <h1 class="text-3xl font-bold text-white mb-2">{{ user.name || user['user-id'] }}</h1>
            <p class="text-gray-400 mb-4">@{{ user['user-id'] }}</p>
            <p v-if="user.email" class="text-gray-500 text-sm mb-4">{{ user.email }}</p>
            
            <div class="flex flex-wrap gap-4 justify-center md:justify-start">
               <!-- Stats / Actions placeholder -->
               <!-- Could show number of followers etc if available -->
            </div>
          </div>
          
          <div v-if="!isCurrentUser" class="flex gap-3">
             <!-- Follow request button would go here -->
             <button class="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition font-medium">
               Follow
             </button>
          </div>
        </div>
      </div>

      <!-- Content -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <!-- Sidebar (Friends/About) -->
        <div class="lg:col-span-1 space-y-6">
           <div class="bg-neutral-900 border border-neutral-800 rounded-xl p-6">
             <h3 class="font-bold text-gray-200 mb-4">About</h3>
             <p class="text-gray-400 text-sm">
                Joined {{ new Date(user['created-at']?.timestamp || Date.now()).toLocaleDateString() }}
             </p>
           </div>
           
           <div v-if="user['connected-users'] && user['connected-users'].length > 0" class="bg-neutral-900 border border-neutral-800 rounded-xl p-6">
             <h3 class="font-bold text-gray-200 mb-4">Connections</h3>
             <div class="space-y-3">
                <div 
                  v-for="conn in user['connected-users']" 
                  :key="conn[0]" 
                  class="flex items-center space-x-3 cursor-pointer hover:bg-neutral-800 p-2 rounded-lg transition"
                  @click="router.push(`/profile/${conn[0]}`)"
                >
                    <div class="w-8 h-8 rounded-full bg-neutral-800 border border-neutral-700 flex items-center justify-center text-xs font-bold text-gray-400">
                        {{ conn[0].charAt(0).toUpperCase() }}
                    </div>
                    <div>
                         <p class="text-sm font-medium text-gray-200">{{ conn[0] }}</p>
                         <p class="text-xs text-gray-500 capitalize">{{ conn[1]['connection-types'].join(', ') }}</p>
                    </div>
                </div>
             </div>
           </div>
        </div>

        <!-- Main Feed -->
        <div class="lg:col-span-2">
           <div v-if="isCurrentUser" class="mb-6">
              <CreatePost @post-created="loadProfile" />
           </div>
           
           <h3 class="text-xl font-bold text-gray-200 mb-4">Posts</h3>
           
           <div v-if="posts.length === 0" class="text-center py-10 bg-neutral-900/50 rounded-xl border border-neutral-800 border-dashed">
             <p class="text-gray-500">No posts yet.</p>
           </div>
           
           <div v-else class="space-y-6">
             <PostCard v-for="post in posts" :key="post['post-id']" :post="post" />
           </div>
        </div>
      </div>
    </div>
  </div>
</template>
