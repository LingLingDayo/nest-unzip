<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { SettingItem } from './types';

import { openUrl } from '@tauri-apps/plugin-opener';

const props = defineProps<{
  config: SettingItem;
  modelValue?: any;
  settings?: any;
}>();

const emit = defineEmits(['update:modelValue']);

const ctrlValue = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
});

// 获取选项值的辅助函数：如果 value 为空，则回退到 label
const getOptValue = (opt: any) => {
  return opt.value === undefined ? opt.label : opt.value;
};

// Select 组件状态与逻辑
const isSelectOpen = ref(false);
const selectTriggerRef = ref<HTMLElement | null>(null);
const placement = ref<'bottom' | 'top'>('bottom');

watch(isSelectOpen, (isOpen) => {
  if (isOpen && selectTriggerRef.value) {
    const rect = selectTriggerRef.value.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    // 下拉列表最大高度为 max-h-60，即 240px，加上间距等，判断 260px 空间
    if (spaceBelow < 260 && rect.top > spaceBelow) {
      placement.value = 'top';
    } else {
      placement.value = 'bottom';
    }
  }
});

const isMultiple = computed(() => {
  return props.config.type === 'select' && props.config.multiple;
});

const selectLabel = computed(() => {
  if (props.config.type !== 'select') return '';
  if (isMultiple.value) {
    if (!Array.isArray(ctrlValue.value) || ctrlValue.value.length === 0) return props.config.placeholder || '';
    return props.config.options.filter(o => ctrlValue.value.includes(getOptValue(o))).map(o => o.label).join(', ');
  } else {
    const opt = props.config.options.find(o => getOptValue(o) === ctrlValue.value);
    return opt ? opt.label : (props.config.placeholder || '');
  }
});

const handleSelect = (val: any) => {
  if (isMultiple.value) {
    let current = Array.isArray(ctrlValue.value) ? [...ctrlValue.value] : [];
    const idx = current.indexOf(val);
    if (idx > -1) {
      current.splice(idx, 1);
    } else {
      current.push(val);
    }
    ctrlValue.value = current;
  } else {
    ctrlValue.value = val;
    isSelectOpen.value = false;
  }
};

// Info 类型文本渲染逻辑
const renderedSegments = computed(() => {
  if (props.config.type !== 'info-text') return [];
  
  let text = props.config.text;
  
  // 1. 变量替换 {{variable}}
  text = text.replace(/\{\{(.*?)\}\}/g, (_, key) => {
    const trimmedKey = key.trim();
    return props.settings?.[trimmedKey] !== undefined ? String(props.settings[trimmedKey]) : `{{${key}}}`;
  });

  // 2. 解析 [label](url) 格式
  const segments: { type: 'text' | 'link'; content: string; url?: string }[] = [];
  const linkRegex = /\[(.*?)\]\((.*?)\)/g;
  let lastIndex = 0;
  let match;

  while ((match = linkRegex.exec(text)) !== null) {
    // 添加链接前的文本
    if (match.index > lastIndex) {
      segments.push({
        type: 'text',
        content: text.substring(lastIndex, match.index)
      });
    }
    
    // 添加链接
    segments.push({
      type: 'link',
      content: match[1],
      url: match[2]
    });
    
    lastIndex = linkRegex.lastIndex;
  }

  // 添加剩余文本
  if (lastIndex < text.length) {
    segments.push({
      type: 'text',
      content: text.substring(lastIndex)
    });
  }

  return segments;
});

const openExternalLink = async (url: string) => {
  try {
    await openUrl(url);
  } catch (err) {
    console.error("Failed to open external link:", err);
    // Fallback to window.open if tauri open fails (e.g. in browser dev mode)
    window.open(url, '_blank');
  }
};
</script>

<template>
  <!-- Slider -->
  <div v-if="config.type === 'slider'" :class="['w-full', config.label ? 'space-y-2' : '']">
    <div class="flex items-center justify-between">
        <label v-if="config.label" :for="config.id" class="block text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</label>
        <span class="text-xs text-center font-black font-mono text-app-primary bg-app-primary-light px-4 py-0.5 rounded-lg border border-app-primary/10 tracking-widest">{{ ctrlValue }}</span>
    </div>
    <p v-if="config.description" class="text-[11px] text-app-text-mute pb-1 leading-relaxed italic opacity-80">{{ config.description }}</p>
    <div class="flex items-center space-x-4">
      <input 
        :id="config.id" 
        type="range" 
        v-model.number="ctrlValue"
        :min="config.min || 0" 
        :max="config.max || 100"
        class="flex-1 w-full h-1 bg-app-border rounded-full appearance-none cursor-pointer accent-app-primary"
      />
    </div>
  </div>

  <!-- Input -->
  <div v-else-if="config.type === 'input'" :class="['w-full', config.label ? 'space-y-2' : '']">
    <label v-if="config.label" :for="config.id" class="block text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</label>
    <p v-if="config.description" class="text-[11px] text-app-text-mute pb-1 leading-relaxed italic opacity-80">{{ config.description }}</p>
    <div class="relative group flex items-center">
        <input 
          v-if="config.inputType === 'number'"
          :id="config.id"
          type="number" 
          v-model.number="ctrlValue"
          :step="config.step || 'any'"
          class="w-full bg-app-surface border border-app-border rounded-xl px-4 py-2.5 text-[13px] text-app-text font-medium placeholder:text-app-text-mute focus:outline-none focus:border-app-primary/50 focus:ring-4 focus:ring-app-primary/5 transition-all shadow-app-sm"
          :placeholder="config.placeholder || ''"
        />
        <input 
          v-else
          :id="config.id"
          :type="config.inputType || 'text'" 
          v-model="ctrlValue"
          class="w-full bg-app-surface border border-app-border rounded-xl px-4 py-2.5 text-[13px] text-app-text font-medium placeholder:text-app-text-mute focus:outline-none focus:border-app-primary/50 focus:ring-4 focus:ring-app-primary/5 transition-all shadow-app-sm"
          :placeholder="config.placeholder || ''"
        />
        <div 
          class="absolute inset-y-0 flex items-center opacity-0 group-focus-within:opacity-30 transition-opacity pointer-events-none"
          :class="config.inputType === 'number' ? 'right-10' : 'right-4'"
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" /></svg>
        </div>
    </div>
  </div>

  <!-- Textarea -->
  <div v-else-if="config.type === 'textarea'" :class="['w-full', config.label ? 'space-y-2' : '']">
    <label v-if="config.label" :for="config.id" class="block text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</label>
    <p v-if="config.description" class="text-[11px] text-app-text-mute pb-1 leading-relaxed italic opacity-80">{{ config.description }}</p>
    <textarea 
      :id="config.id"
      v-model="ctrlValue"
      :rows="config.rows || 3"
      class="w-full bg-app-surface border border-app-border rounded-2xl px-4 py-2.5 text-[13px] text-app-text font-medium placeholder:text-app-text-mute focus:outline-none focus:border-app-primary/50 focus:ring-4 focus:ring-app-primary/5 transition-all shadow-app-sm resize-y custom-scrollbar leading-relaxed"
      :placeholder="config.placeholder || ''"
    ></textarea>
  </div>
  
  <!-- Select-->
  <div v-else-if="config.type === 'select'" :class="['w-full', config.label ? 'space-y-2' : '']">
    <label v-if="config.label" :for="config.id" class="block text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</label>
    <p v-if="config.description" class="text-[11px] text-app-text-mute pb-1 leading-relaxed italic opacity-80">{{ config.description }}</p>
    <div class="relative">
      <div 
        ref="selectTriggerRef"
        @click="isSelectOpen = !isSelectOpen"
        class="w-full bg-app-surface border border-app-border rounded-xl px-4 py-2.5 text-[13px] font-medium shadow-app-sm cursor-pointer transition-all flex items-center justify-between"
        :class="[
          isSelectOpen ? 'border-app-primary/50 ring-4 ring-app-primary/5' : 'hover:border-app-border-focus',
          !selectLabel ? 'text-app-text-mute' : 'text-app-text'
        ]"
      >
        <span class="truncate pr-4">{{ selectLabel || config.placeholder || '请选择' }}</span>
        
        <div class="shrink-0 transition-transform duration-300 text-app-text-mute" :class="{ 'rotate-180': isSelectOpen }">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
      
      <!-- 遮罩层，用于点击外部关闭 -->
      <div v-if="isSelectOpen" class="fixed inset-0 z-40" @click="isSelectOpen = false"></div>
      
      <!-- 下拉列表 -->
      <transition 
        :enter-active-class="`transition ease-out duration-200 ${placement === 'top' ? 'origin-bottom' : 'origin-top'}`" 
        enter-from-class="opacity-0 scale-y-95" 
        enter-to-class="opacity-100 scale-y-100" 
        :leave-active-class="`transition ease-in duration-150 ${placement === 'top' ? 'origin-bottom' : 'origin-top'}`" 
        leave-from-class="opacity-100 scale-y-100" 
        leave-to-class="opacity-0 scale-y-95"
      >
        <div 
          v-if="isSelectOpen"
          class="absolute z-50 w-full bg-app-surface border border-app-border rounded-xl shadow-app-xl py-2 max-h-60 overflow-y-auto custom-scrollbar"
          :class="[
            placement === 'top' ? 'bottom-full mb-2' : 'top-full mt-2'
          ]"
          :style="{ transformOrigin: placement === 'top' ? 'bottom' : 'top' }"
        >
          <div 
            v-for="opt in (config.type === 'select' ? config.options : [])" 
            :key="getOptValue(opt)"
            @click="handleSelect(getOptValue(opt))"
            class="px-4 py-2.5 text-[13px] font-medium cursor-pointer transition-colors flex items-center justify-between group hover:bg-app-bg"
            :class="[
               (isMultiple ? (Array.isArray(ctrlValue) && ctrlValue.includes(getOptValue(opt))) : ctrlValue === getOptValue(opt))
                 ? 'text-app-primary bg-app-primary-light/30' 
                 : 'text-app-text'
            ]"
          >
            <span class="truncate group-hover:text-app-primary transition-colors">{{ opt.label }}</span>
            <svg 
              v-if="(isMultiple ? (Array.isArray(ctrlValue) && ctrlValue.includes(getOptValue(opt))) : ctrlValue === getOptValue(opt))"
              xmlns="http://www.w3.org/2000/svg" 
              class="h-4 w-4 shrink-0 text-app-primary" 
              fill="none" 
              viewBox="0 0 24 24" 
              stroke="currentColor"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
            </svg>
          </div>
          
          <div v-if="config.type === 'select' && (!config.options || config.options.length === 0)" class="px-4 py-3 text-[13px] text-app-text-mute text-center italic">
            无可用选项
          </div>
        </div>
      </transition>
    </div>
  </div>

  <!-- Switch -->
  <label v-else-if="config.type === 'switch'" class="flex items-center justify-between cursor-pointer group py-3 px-4 bg-app-bg rounded-2xl border border-app-border/40 hover:border-app-primary/30 transition-all duration-500">
    <div class="flex flex-col pr-6">
      <span v-if="config.label" class="text-[12px] font-black tracking-widest text-app-text-dim group-hover:text-app-text transition-colors">{{ config.label }}</span>
      <span v-if="config.description" class="text-[11px] text-app-text-mute italic mt-1 leading-relaxed opacity-80">{{ config.description }}</span>
    </div>
    <div class="relative shrink-0">
      <input type="checkbox" v-model="ctrlValue" class="sr-only peer">
      <div class="w-10 h-5.5 bg-app-border rounded-full peer peer-checked:bg-app-primary transition-all after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4.5 after:w-4.5 after:transition-all after:shadow-app-sm peer-checked:after:translate-x-4.5 animate-all duration-300"></div>
    </div>
  </label>

  <!-- Radio -->
  <div v-else-if="config.type === 'radio'" :class="[config.label ? 'space-y-2' : '']">
    <label v-if="config.label" class="block text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</label>
    <p v-if="config.description" class="text-[11px] text-app-text-mute pb-1 leading-relaxed italic opacity-80">{{ config.description }}</p>
    <div class="flex flex-wrap gap-4">
      <label v-for="opt in config.options" :key="getOptValue(opt)" class="flex items-center space-x-2.5 cursor-pointer group">
        <div class="relative w-4.5 h-4.5 flex items-center justify-center">
            <input type="radio" :value="getOptValue(opt)" v-model="ctrlValue" class="w-full h-full accent-app-primary bg-app-surface border-app-border transition-all">
        </div>
        <span class="text-xs font-black tracking-widest text-app-text-mute group-hover:text-app-text transition-colors">{{ opt.label }}</span>
      </label>
    </div>
  </div>

  <!-- Button -->
  <div v-else-if="config.type === 'button'" :class="[config.label ? 'space-y-3' : '']">
    <div class="flex items-center justify-between">
      <div class="flex flex-col pr-6">
        <span v-if="config.label" class="text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</span>
        <span v-if="config.description" class="text-[11px] text-app-text-mute italic mt-1 leading-relaxed opacity-80">{{ config.description }}</span>
      </div>
      <button 
        @click="config.type === 'button' && config.onClick?.(settings)"
        class="shrink-0 px-6 py-2.5 font-black text-xs uppercase tracking-[0.2em] rounded-xl transition-all shadow-app-sm border border-app-border hover:bg-app-primary hover:text-white hover:border-transparent active:scale-95 cursor-pointer"
        :class="(config.type === 'button' && config.colorClass) || 'text-app-text bg-app-surface'"
      >
        {{ config.type === 'button' ? config.buttonText : '' }}
      </button>
    </div>
  </div>

  <!-- Checkbox -->
  <div v-else-if="config.type === 'checkbox'" :class="[config.label ? 'space-y-3' : '']">
    <label v-if="config.label" class="block text-[12px] font-black tracking-widest text-app-text-dim">{{ config.label }}</label>
    <p v-if="config.description" class="text-[11px] text-app-text-mute pb-1 leading-relaxed italic opacity-80">{{ config.description }}</p>
    
    <!-- Grid Layout Mode -->
    <div v-if="config.layout === 'grid'" 
      class="grid gap-2"
      :style="{ gridTemplateColumns: `repeat(${config.columns || 4}, minmax(0, 1fr))` }"
    >
      <label 
        v-for="opt in config.options" 
        :key="getOptValue(opt)" 
        class="flex items-center space-x-2 px-3 py-2 rounded-xl border transition-all cursor-pointer group shadow-app-sm"
        :class="Array.isArray(ctrlValue) && ctrlValue.includes(getOptValue(opt)) 
          ? 'bg-app-primary-light border-app-primary/40 ring-2 ring-app-primary/5' 
          : 'bg-app-surface border-app-border hover:border-app-border-focus hover:bg-app-bg'"
        :title="opt.label"
      >
        <input type="checkbox" :value="getOptValue(opt)" v-model="ctrlValue" class="w-3.5 h-3.5 accent-app-primary bg-app-surface border-app-border rounded">
        <span class="text-[10px] font-black tracking-widest text-app-text-mute group-hover:text-app-text truncate transition-colors">{{ opt.label }}</span>
      </label>
    </div>

    <!-- Default Multi-line wrap mode -->
    <div v-else class="flex flex-wrap gap-6">
      <label v-for="opt in config.options" :key="getOptValue(opt)" class="flex items-center space-x-2.5 cursor-pointer group">
        <input type="checkbox" :value="getOptValue(opt)" v-model="ctrlValue" class="w-4 h-4 accent-app-primary bg-app-surface border-app-border rounded transition-all">
        <span class="text-[11px] font-black tracking-widest text-app-text-mute group-hover:text-app-text transition-colors">{{ opt.label }}</span>
      </label>
    </div>
  </div>

  <!-- Info -->
  <div v-else-if="config.type === 'info-text'" :class="[config.label ? 'space-y-2' : '']" class="bg-app-bg/40 p-4 rounded-2xl border border-app-border/30">
    <label v-if="config.label" class="block text-[12px] font-black tracking-widest text-app-text-dim mb-1">{{ config.label }}</label>
    <div class="text-[12px] leading-relaxed text-app-text-mute font-medium">
      <template v-for="(seg, idx) in renderedSegments" :key="idx">
        <span v-if="seg.type === 'text'">{{ seg.content }}</span>
        <a 
          v-else-if="seg.type === 'link'" 
          href="javascript:void(0)" 
          @click="openExternalLink(seg.url!)"
          class="text-app-primary hover:underline font-bold transition-all mx-0.5"
        >{{ seg.content }}</a>
      </template>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { 
    width: 4px; 
    cursor: default;
}
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { 
    background: var(--color-app-border); 
    border-radius: 10px;
    cursor: default;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: var(--color-app-text-mute); }
</style>
