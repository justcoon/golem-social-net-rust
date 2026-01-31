<script setup lang="ts">
import { computed } from 'vue';
import type { Post } from '../api';
import { useRouter } from 'vue-router';

const props = defineProps<{
  post: Post;
}>();

const router = useRouter();

const formattedDate = computed(() => {
  const createdAt = props.post['created-at'];
  const dateStr = (typeof createdAt === 'object' && createdAt.timestamp) 
      ? createdAt.timestamp 
      : (createdAt as string);

  return new Date(dateStr).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
});

function navigateToAuthor() {
  router.push(`/profile/${props.post['created-by']}`);
}
</script>

<template>
  <div class="bg-neutral-900 border border-neutral-800 rounded-xl p-6 shadow-lg hover:shadow-purple-900/10 transition duration-300">
    <div class="flex items-start justify-between mb-4">
      <div class="flex items-center space-x-3 cursor-pointer" @click="navigateToAuthor">
        <div class="w-10 h-10 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white font-bold select-none">
          {{ props.post['created-by'].charAt(0).toUpperCase() }}
        </div>
        <div>
          <h3 class="font-medium text-gray-200 hover:text-purple-400 transition">{{ props.post['created-by'] }}</h3>
          <p class="text-xs text-gray-500">{{ formattedDate }}</p>
        </div>
      </div>
    </div>
    
    <div class="text-gray-300 whitespace-pre-wrap leading-relaxed">
      {{ props.post.content }}
    </div>
    
    <!-- Future interactions (like/comment) could go here -->
  </div>
</template>
