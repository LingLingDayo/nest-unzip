import type { SettingGroup } from './types';

/**
 * 从 SettingGroup 数组中提取所有属性的默认值，并组成对象
 * @param groups SettingGroup 数组
 * @returns 包含所有 ID 对应默认值的对象
 */
export const extractDefaultSettings = (groups: SettingGroup[]): Record<string, any> => {
  const defaults: Record<string, any> = {};
  
  groups.forEach(group => {
    group.items.forEach(item => {
      if (item.defaultValue !== undefined) {
        defaults[item.id] = item.defaultValue;
      }
    });
  });
  
  return defaults;
};
