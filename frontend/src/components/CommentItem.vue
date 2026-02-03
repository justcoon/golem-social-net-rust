<script setup lang="ts">
import { ref, computed } from 'vue';
import { type Comment, type LikeType, api } from '../api';
import LikeReactions from './LikeReactions.vue';
import { useUserStore } from '../stores/user';
import { storeToRefs } from 'pinia';

const props = defineProps<{
  comment: Comment;
  allComments: Comment[];
  postId: string;
  depth: number;
}>();

const emit = defineEmits<{
  (e: 'comment-added', newComment: Comment): void;
}>();

const userStore = useUserStore();
const { userId, isLoggedIn } = storeToRefs(userStore);

const isReplying = ref(false);
const replyContent = ref('');
const isSubmitting = ref(false);

const childComments = computed(() => {
  return props.allComments.filter(c => c['parent-comment-id'] === props.comment['comment-id'])
    .sort((a, b) => new Date(a['created-at'].timestamp).getTime() - new Date(b['created-at'].timestamp).getTime());
});

async function handleLike(type: LikeType) {
  if (!userId.value) return;
  const uid = userId.value;
  
  if (!props.comment.likes) props.comment.likes = [];
  const likes = props.comment.likes;
  
  const existingEntry = likes.find(([u]) => u === uid);
  const oldLike = existingEntry ? existingEntry[1] : undefined;
  
  if (existingEntry) {
    existingEntry[1] = type;
  } else {
    likes.push([uid, type]);
  }
  
  try {
    await api.likeComment(props.postId, props.comment['comment-id'], uid, type);
  } catch (error) {
    console.error('Failed to like comment:', error);
    const currentIndex = likes.findIndex(([u]) => u === uid);
    if (currentIndex !== -1) {
      const entry = likes[currentIndex];
      if (entry) {
        if (oldLike) {
          entry[1] = oldLike;
        } else {
          likes.splice(currentIndex, 1);
        }
      }
    }
  }
}

async function handleUnlike() {
  if (!userId.value || !props.comment.likes) return;
  const uid = userId.value;
  const likes = props.comment.likes;
  
  const existingIndex = likes.findIndex(([u]) => u === uid);
  if (existingIndex === -1) return;
  
  const entry = likes[existingIndex];
  if (!entry) return;
  
  const oldLike = entry[1];
  likes.splice(existingIndex, 1);
  
  try {
    await api.unlikeComment(props.postId, props.comment['comment-id'], uid);
  } catch (error) {
    console.error('Failed to unlike comment:', error);
    likes.push([uid, oldLike]);
  }
}

async function submitReply() {
  if (!replyContent.value.trim() || !userId.value) return;
  
  isSubmitting.value = true;
  try {
    const response = await api.addComment(props.postId, userId.value, replyContent.value, props.comment['comment-id']);
    
    const newCommentObj: Comment = {
      'comment-id': response.data.ok,
      content: replyContent.value,
      'created-by': userId.value,
      'created-at': { timestamp: new Date().toISOString() },
      'parent-comment-id': props.comment['comment-id'],
      likes: []
    };
    
    emit('comment-added', newCommentObj);
    replyContent.value = '';
    isReplying.value = false;
  } catch (error) {
    console.error('Failed to post reply:', error);
  } finally {
    isSubmitting.value = false;
  }
}
</script>

<template>
  <div class="comment-item" :style="{ marginLeft: depth > 0 ? '1.5rem' : '0' }">
    <div class="bg-neutral-800/50 rounded-lg p-4 mb-3 border-l-2 border-transparent hover:border-purple-500/30 transition-colors">
      <div class="flex justify-between items-start mb-2">
        <span class="text-xs font-bold text-gray-300">{{ comment['created-by'] }}</span>
        <span class="text-[10px] text-gray-600">{{ new Date(comment['created-at'].timestamp).toLocaleString() }}</span>
      </div>
      <p class="text-sm text-gray-400 whitespace-pre-wrap mb-3">{{ comment.content }}</p>
      
      <div class="flex items-center space-x-4">
        <!-- Comment Likes -->
        <LikeReactions 
          :likes="comment.likes" 
          :current-user-id="userId"
          @like="handleLike"
          @unlike="handleUnlike"
        />
        
        <button 
          v-if="isLoggedIn && depth < 5" 
          @click="isReplying = !isReplying"
          class="text-[10px] text-purple-400 hover:text-purple-300 transition font-medium"
        >
          {{ isReplying ? 'Cancel' : 'Reply' }}
        </button>
      </div>

      <!-- Reply Input -->
      <div v-if="isReplying" class="mt-3 flex gap-2">
        <input 
          v-model="replyContent"
          type="text" 
          placeholder="Write a reply..." 
          class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-xs text-gray-200 focus:ring-1 focus:ring-purple-500 placeholder-gray-600 outline-none"
          @keyup.enter="submitReply"
        />
        <button 
          @click="submitReply" 
          :disabled="!replyContent.trim() || isSubmitting"
          class="bg-purple-600 hover:bg-purple-700 text-white text-[10px] font-medium px-3 py-1 rounded disabled:opacity-50 disabled:cursor-not-allowed transition"
        >
          Post
        </button>
      </div>
    </div>

    <!-- Nested Replies -->
    <div v-if="childComments.length > 0" class="nested-replies">
      <CommentItem 
        v-for="child in childComments" 
        :key="child['comment-id']"
        :comment="child"
        :all-comments="allComments"
        :post-id="postId"
        :depth="depth + 1"
        @comment-added="(nc) => emit('comment-added', nc)"
      />
    </div>
  </div>
</template>

<style scoped>
.comment-item {
  position: relative;
}

.nested-replies::before {
  content: '';
  position: absolute;
  left: -0.75rem;
  top: 0;
  bottom: 1.5rem;
  width: 1px;
  background: rgba(139, 92, 246, 0.1);
}
</style>
