<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { type Post, type Comment, api } from '../api';
import { useRouter } from 'vue-router';
import { useUserStore } from '../stores/user';
import { storeToRefs } from 'pinia';

const props = defineProps<{
  post: Post;
}>();

const router = useRouter();
const userStore = useUserStore();
const { userId, isLoggedIn } = storeToRefs(userStore);

const newComment = ref('');
const isSubmitting = ref(false);

const comments = ref<Comment[]>(
    props.post.comments ? Object.values(props.post.comments.map(([_, comment]) => comment)) : []
);

// Watch for prop updates to update local comments if post data changes externally (e.g. parent refetch)
watch(() => props.post.comments, (newComments) => {
    console.log('PostCard: comments updated', newComments);
    if (newComments) {
        comments.value = Object.values(newComments);
    }
}, { deep: true });


const sortedComments = computed(() => {
  return [...comments.value].sort((a, b) => {
    const timeA = typeof a['created-at'] === 'object' ? a['created-at'].timestamp : a['created-at'];
    const timeB = typeof b['created-at'] === 'object' ? b['created-at'].timestamp : b['created-at'];
    return new Date(timeA).getTime() - new Date(timeB).getTime();
  });
});

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

async function submitComment() {
    if (!newComment.value.trim() || !userId.value) return;

    isSubmitting.value = true;
    try {
        const response = await api.addComment(props.post['post-id'], userId.value, newComment.value);
        
        // Optimistic update
        const createdNow = new Date().toISOString();
        const newCommentObj: Comment = {
            'comment-id': response.data, // Assuming backend returns the ID string
            content: newComment.value,
            'created-by': userId.value,
            'created-at': { timestamp: createdNow }
        };
        
        comments.value.push(newCommentObj);
        newComment.value = '';
    } catch (error) {
        console.error('Failed to post comment:', error);
    } finally {
        isSubmitting.value = false;
    }
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

    <div class="mt-4 border-t border-neutral-800 pt-4">
      <h4 class="text-sm font-semibold text-gray-400 mb-3">Comments</h4>
      
      <div v-if="comments.length === 0" class="text-xs text-gray-600 italic mb-4">
        No comments yet.
      </div>
      
      <div class="space-y-3 mb-4">
        <div v-for="comment in sortedComments" :key="comment['comment-id']" class="bg-neutral-800/50 rounded p-3">
            <div class="flex justify-between items-start mb-1">
                <span class="text-xs font-bold text-gray-300">{{ comment['created-by'] }}</span>
                <span class="text-[10px] text-gray-600">{{ new Date(typeof comment['created-at'] === 'object' ? comment['created-at'].timestamp : comment['created-at']).toLocaleString() }}</span>
            </div>
            <p class="text-sm text-gray-400 whitespace-pre-wrap">{{ comment.content }}</p>
        </div>
      </div>

      <div v-if="isLoggedIn" class="flex gap-2">
        <input 
            v-model="newComment"
            type="text" 
            placeholder="Write a comment..." 
            class="flex-1 bg-neutral-800 border-none rounded px-3 py-2 text-sm text-gray-200 focus:ring-1 focus:ring-purple-500 placeholder-gray-600"
            @keyup.enter="submitComment"
        />
        <button 
            @click="submitComment" 
            :disabled="!newComment.trim() || isSubmitting"
            class="bg-purple-600 hover:bg-purple-700 text-white text-xs font-medium px-4 py-2 rounded disabled:opacity-50 disabled:cursor-not-allowed transition"
        >
            {{ isSubmitting ? 'Posting...' : 'Post' }}
        </button>
      </div>
       <div v-else class="text-xs text-gray-600">
        <router-link to="/login" class="text-purple-400 hover:underline">Log in</router-link> to comment.
      </div>
    </div>
  </div>
</template>
