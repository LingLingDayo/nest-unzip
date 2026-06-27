<script setup lang="ts">
import { ref, watch, reactive, computed } from 'vue';
import DynamicControl from './DynamicControl.vue';
import type { SettingGroup, SettingItem } from './types';

const props = defineProps<{
  show: boolean;
  settings: any;
  groups: SettingGroup[];
  width?: string;
  height?: string;
  maxWidth?: string;
  maxHeight?: string;
}>();

const emit = defineEmits(['update:show', 'update:settings', 'save', 'cancel']);

const localSettings = reactive({ ...props.settings });
const activeTabId = ref(props.groups[0]?.id || '');

const contentScrollBox = ref<HTMLElement | null>(null);

watch(() => props.show, (newVal) => {
  if (newVal) {
    Object.assign(localSettings, props.settings);
    if (!activeTabId.value && props.groups.length > 0) {
      activeTabId.value = props.groups[0].id;
    }
  }
});

watch(activeTabId, () => {
  if (contentScrollBox.value) {
    contentScrollBox.value.scrollTop = 0;
  }
});

const currentGroup = computed(() => {
  return props.groups.find(g => g.id === activeTabId.value);
});

const handleSave = () => {
    emit('update:settings', { ...localSettings });
    emit('save', { ...localSettings });
    emit('update:show', false);
};

const handleCancel = () => {
    emit('cancel');
    emit('update:show', false);
};

// 根据 item.visible 判断是否渲染该控件
const isItemVisible = (item: SettingItem): boolean => {
  if (item.visible === undefined) return true;
  if (typeof item.visible === 'function') return item.visible(localSettings);
  return !!item.visible;
};
</script>

<template>
  <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-app-text/20 backdrop-blur-md transition-all animate-in fade-in duration-300" @click.self="handleCancel">
    <div 
      class="bg-app-surface border border-app-border rounded-[32px] shadow-app-xl w-full max-w-4xl max-h-[90vh] flex flex-col transform transition-all overflow-hidden animate-in zoom-in-95 duration-500"
      :style="{ width, height, maxWidth, maxHeight }"
    >
      <!-- Header Area -->
      <div class="px-7 py-4 border-b border-app-border flex justify-between items-center bg-app-surface shrink-0">
        <div class="flex flex-col">
            <h3 class="text-2xl font-black text-app-text tracking-tight flex items-center">
              设置 <span class="ml-3 font-medium opacity-20 text-sm tracking-[0.2em]">SETTINGS</span>
            </h3>
        </div>
        <button @click="handleCancel" class="text-app-text-mute hover:text-app-text transition-all p-2 rounded-2xl hover:bg-app-bg cursor-pointer group">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 group-hover:rotate-90 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Main Body with Sidebar -->
      <div class="flex-1 flex overflow-hidden">
        <!-- Sidebar Navigation -->
        <div class="w-60 border-r border-app-border bg-app-bg/30 overflow-y-auto shrink-0 py-4 px-4 space-y-2 custom-scrollbar">
          <button 
            v-for="group in groups" 
            :key="group.id"
            @click="activeTabId = group.id"
            class="w-full flex items-center px-4 py-3.5 rounded-2xl transition-all duration-300 group/nav relative overflow-hidden"
            :class="[
              activeTabId === group.id 
                ? 'bg-app-surface shadow-sm text-app-text' 
                : 'text-app-text-mute hover:text-app-text hover:bg-app-surface/50'
            ]"
          >
            <!-- Active Indicator -->
            <div 
              class="absolute left-0 top-3 bottom-3 w-1.5 rounded-r-full transition-all duration-500"
              :style="{ 
                backgroundColor: group.color || 'var(--color-app-primary)',
                transform: activeTabId === group.id ? 'scaleY(1)' : 'scaleY(0)',
                opacity: activeTabId === group.id ? 1 : 0
              }"
            ></div>

            <div class="flex items-center gap-3">
              <span 
                class="w-2 h-2 rounded-full transition-all duration-300" 
                :style="{ 
                  backgroundColor: group.color || 'var(--color-app-primary)',
                  boxShadow: activeTabId === group.id ? `0 0 12px ${group.color || 'var(--color-app-primary)'}` : 'none',
                  opacity: activeTabId === group.id ? 1 : 0.4
                }"
              ></span>
              <span class="font-bold text-sm tracking-wide uppercase">{{ group.title }}</span>
            </div>

            <!-- Hover Decoration -->
            <div 
              v-if="activeTabId !== group.id"
              class="absolute inset-0 bg-current opacity-0 group-hover/nav:opacity-[0.03] transition-opacity pointer-events-none"
            ></div>
          </button>
        </div>

        <!-- Content Area -->
        <div ref="contentScrollBox" class="flex-1 overflow-y-auto p-8 custom-scrollbar bg-app-surface/20">
          <div v-if="currentGroup" :key="activeTabId" class="max-w-2xl mx-auto animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div class="mb-8">
              <div class="flex items-center gap-4 mb-2">
                <div class="h-1 w-12 rounded-full" :style="{ backgroundColor: currentGroup.color || 'var(--color-app-primary)' }"></div>
                <h4 class="text-[12px] font-black uppercase tracking-[0.3em] opacity-40">Section</h4>
              </div>
              <h2 class="text-3xl font-black text-app-text tracking-tight">{{ currentGroup.title }}</h2>
              <p v-if="currentGroup.description" class="mt-2 text-app-text-mute text-sm leading-relaxed">{{ currentGroup.description }}</p>
            </div>

            <div class="space-y-8">
              <template v-for="item in currentGroup.items" :key="item.id">
                <div v-if="isItemVisible(item)" class="group/item">
                  <DynamicControl :config="item" :settings="localSettings" v-model="localSettings[item.id]" />
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>

      <!-- Action Footer -->
      <div class="px-9 py-5 border-t border-app-border flex justify-end shrink-0 gap-4 bg-app-surface">
        <button 
          @click="handleCancel" 
          class="px-8 py-3 text-app-text-dim hover:text-app-text font-black text-xs uppercase tracking-[0.2em] rounded-2xl transition-all border border-app-border hover:bg-app-bg cursor-pointer shadow-sm active:scale-95"
        >
          取消
        </button>
        <button 
          @click="handleSave" 
          class="px-11 py-3 bg-app-text text-app-bg hover:bg-app-primary hover:text-white font-black text-xs uppercase tracking-[0.2em] rounded-2xl shadow-xl shadow-app-primary/10 transition-all active:scale-95 cursor-pointer"
        >
          保存所有更改
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 5px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { 
    background: var(--color-app-border); 
    border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: var(--color-app-text-mute); }

.animate-in {
  animation-fill-mode: both;
}

@keyframes slide-in-from-bottom {
  from {
    transform: translateY(16px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

.slide-in-from-bottom-4 {
  animation: slide-in-from-bottom 0.6s cubic-bezier(0.16, 1, 0.3, 1);
}
</style>
