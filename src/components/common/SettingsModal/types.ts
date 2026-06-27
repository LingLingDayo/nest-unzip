export type SettingControlType = 'slider' | 'input' | 'textarea' | 'switch' | 'radio' | 'checkbox' | 'button' | 'select' | 'info-text';

export interface SettingItemBase {
  id: string;
  type: SettingControlType;
  label?: string;
  description?: string;
  visible?: boolean | ((settings: any) => boolean);
  defaultValue?: any;
}

export interface SliderSettingItem extends SettingItemBase {
  type: 'slider';
  min?: number;
  max?: number;
  step?: number;
}

export interface InputSettingItem extends SettingItemBase {
  type: 'input';
  inputType?: 'text' | 'number' | 'password';
  placeholder?: string;
  step?: number | string;
}

export interface TextareaSettingItem extends SettingItemBase {
  type: 'textarea';
  placeholder?: string;
  rows?: number;
}

export interface SwitchSettingItem extends SettingItemBase {
  type: 'switch';
}

export interface SettingOption {
  label: string;
  value?: any;
}

export interface RadioSettingItem extends SettingItemBase {
  type: 'radio';
  options: SettingOption[];
}

export interface CheckboxSettingItem extends SettingItemBase {
  type: 'checkbox';
  options: SettingOption[];
  layout?: 'grid' | 'flex';
  columns?: number;
}

export interface SelectSettingItem extends SettingItemBase {
  type: 'select';
  options: SettingOption[];
  multiple?: boolean;
  placeholder?: string;
}

export interface ButtonSettingItem extends SettingItemBase {
  type: 'button';
  buttonText: string;
  onClick?: (settings: any) => void;
  colorClass?: string;
}

export interface InfoTextSettingItem extends SettingItemBase {
  type: 'info-text';
  text: string;
}

export type SettingItem = 
  | SliderSettingItem 
  | InputSettingItem 
  | TextareaSettingItem 
  | SwitchSettingItem 
  | RadioSettingItem 
  | CheckboxSettingItem
  | SelectSettingItem
  | ButtonSettingItem
  | InfoTextSettingItem;

export interface SettingGroup {
  id: string;
  title: string;
  description?: string;
  color?: string;
  colorClass?: string;
  items: SettingItem[];
}
