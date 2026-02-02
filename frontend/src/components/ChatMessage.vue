<script setup lang="ts">
import { computed } from 'vue';
import { type Message, type LikeType, type UserLikeTuple } from '../api';
import LikeReactions from './LikeReactions.vue';

const props = defineProps<{
  message: Message;
  currentUserId: string | null;
}>();

const emit = defineEmits<{
  (e: 'like', type: LikeType): void;
  (e: 'unlike'): void;
}>();

const isOwnMessage = computed(() => props.message['created-by'] === props.currentUserId);

const formattedDate = computed(() => {
  return new Date(props.message['created-at'].timestamp).toLocaleString(undefined, {
    timeStyle: 'short',
  });
});

const likesTuple = computed<UserLikeTuple[]>(() => {
  return props.message.likes;
});
</script>

<template>
  <div class="flex flex-col mb-4" :class="isOwnMessage ? 'items-end' : 'items-start'">
    <div class="flex items-center space-x-2 mb-1">
      <span class="text-xs font-bold text-gray-500">{{ message['created-by'] }}</span>
      <span class="text-[10px] text-gray-600">{{ formattedDate }}</span>
    </div>
    
    <div 
      class="max-w-[80%] rounded-2xl p-3 shadow-sm relative group"
      :class="isOwnMessage ? 'bg-purple-600 text-white rounded-tr-none' : 'bg-neutral-800 text-gray-200 rounded-tl-none'"
    >
      <div class="text-sm whitespace-pre-wrap leading-relaxed">
        {{ message.content }}
      </div>
      
      <!-- Reactions overlay/footer -->
      <div class="mt-2" v-if="likesTuple.length > 0 || !isOwnMessage">
        <LikeReactions 
          :likes="likesTuple" 
          :current-user-id="currentUserId"
          @like="(type) => emit('like', type)"
          @unlike="() => emit('unlike')"
        />
      </div>
    </div>
  </div>
</template>
