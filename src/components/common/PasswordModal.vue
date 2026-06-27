<script setup lang="ts">
import { ref, nextTick } from 'vue';

const isVisible = ref(false);
const title = ref('输入密码');
const message = ref('');
const placeholder = ref('请输入解压密码');
const password = ref('');
const isPasswordVisible = ref(false);
const inputRef = ref<HTMLInputElement | null>(null);

let resolvePromise: ((value: string | null) => void) | null = null;

const open = (opts: { title?: string; message: string; placeholder?: string }) => {
  title.value = opts.title || '输入解压密码';
  message.value = opts.message;
  placeholder.value = opts.placeholder || '请输入解压密码';
  password.value = '';
  isVisible.value = true;
  isPasswordVisible.value = false;
  
  nextTick(() => {
    if (inputRef.value) {
      inputRef.value.focus();
    }
  });

  return new Promise<string | null>((resolve) => {
    resolvePromise = resolve;
  });
};

const handleConfirm = () => {
  isVisible.value = false;
  if (resolvePromise) {
    resolvePromise(password.value);
  }
};

const handleCancel = () => {
  isVisible.value = false;
  if (resolvePromise) {
    resolvePromise(null);
  }
};

defineExpose({
  open
});
</script>

<template>
  <div v-if="isVisible" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-app-text/20 backdrop-blur-md transition-all animate-in fade-in duration-300" @click.self="handleCancel">
    <div class="bg-app-surface border border-app-border rounded-[28px] shadow-app-xl w-full max-w-md flex flex-col transform transition-all overflow-hidden animate-in zoom-in-95 duration-500">
      
      <!-- Header Area -->
      <div class="px-6 py-4 border-b border-app-border flex justify-between items-center bg-app-surface shrink-0">
        <h3 class="text-sm font-black text-app-text tracking-tight flex items-center gap-2">
          <span class="w-2.5 h-2.5 rounded-full bg-app-primary shadow-[0_0_8px_var(--color-app-primary)]"></span>
          {{ title }}
        </h3>
        <button @click="handleCancel" class="text-app-text-mute hover:text-app-rose transition-all p-1.5 rounded-xl hover:bg-app-bg cursor-pointer flex items-center justify-center">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Content/Body -->
      <div class="p-6 flex flex-col gap-4 bg-app-bg/5">
        <p class="text-xs text-app-text-dim leading-relaxed whitespace-pre-wrap font-medium">
          {{ message }}
        </p>

        <!-- Input Field Container -->
        <div class="relative flex items-center">
          <input
            ref="inputRef"
            :type="isPasswordVisible ? 'text' : 'password'"
            v-model="password"
            :placeholder="placeholder"
            class="w-full bg-app-surface border border-app-border rounded-xl pl-4 pr-10 py-3 text-xs font-medium placeholder:text-app-text-mute focus:outline-none focus:border-app-primary/60 focus:ring-4 focus:ring-app-primary-light transition-all"
            @keydown.enter="handleConfirm"
            @keydown.esc="handleCancel"
          />
          <!-- Eye icon -->
          <button 
            @click="isPasswordVisible = !isPasswordVisible" 
            class="absolute right-3 text-app-text-mute hover:text-app-primary transition-colors cursor-pointer flex items-center justify-center"
            type="button"
          >
            <svg v-if="isPasswordVisible" xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l18 18" />
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Action Footer -->
      <div class="px-6 py-4 border-t border-app-border flex justify-end gap-3 bg-app-surface shrink-0">
        <button
          @click="handleCancel"
          class="px-5 py-2 text-app-text-dim hover:text-app-text font-black text-xs rounded-xl transition-all border border-app-border hover:bg-app-bg cursor-pointer shadow-sm active:scale-95"
        >
          取消 (结束解压)
        </button>
        <button
          @click="handleConfirm"
          class="px-6 py-2 bg-gradient-to-r from-app-primary to-app-purple text-white hover:opacity-95 font-black text-xs rounded-xl shadow-lg shadow-app-primary/10 transition-all active:scale-95 cursor-pointer"
        >
          确认解压
        </button>
      </div>

    </div>
  </div>
</template>

<style scoped>
.animate-in {
  animation-fill-mode: both;
}

@keyframes slide-in {
  from {
    transform: translateY(8px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

.zoom-in-95 {
  animation: slide-in 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
</style>
