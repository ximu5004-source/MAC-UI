import { Icon } from "libs/ui/react/components/Icon";
import { ConfigProvider, Tooltip } from "antd";
import type { ComponentChildren } from "preact";
import type { IconName } from "libs/ui/icons";

import cs from "./index.module.css";
import { cx } from "libs/ui/react/utils/styling";

interface Props {
  children: React.ReactNode;
  title?: React.ReactNode;
  description?: string;
  icon?: IconName;
}

export const SettingsGroup = ({ children, title, description, icon }: Props) => {
  return (
    <div className={cs.group} data-settings-card>
      {title && (
        <div className={cs.heading}>
          {icon && (
            <span className={cs.groupIcon} aria-hidden="true">
              <Icon iconName={icon} size={19} />
            </span>
          )}
          <div>
            <h3>{title}</h3>
            {description && <p>{description}</p>}
          </div>
        </div>
      )}
      <div className={cs.content}>{children}</div>
    </div>
  );
};

interface SubGroupProps {
  children: React.ReactNode;
  label: React.ReactNode;
  disabled?: boolean;
}

export const SettingsSubGroup = ({ children, label, disabled }: SubGroupProps) => {
  return (
    <div className={cs.subgroup}>
      <div data-setting className={cs.subtitle}>{label}</div>
      <ConfigProvider componentDisabled={disabled}>
        <div className={cs.content}>{children}</div>
      </ConfigProvider>
    </div>
  );
};

type OptionProps = {
  label?: ComponentChildren;
  tip?: ComponentChildren;
  description?: ComponentChildren;
  action?: ComponentChildren;
  children?: ComponentChildren;
  disabled?: boolean;
  wide?: boolean;
};

export const SettingsOption = (props: OptionProps) => {
  return (
    <div data-setting className={cx(cs.setting, { [cs.disabled!]: props.disabled, [cs.wide!]: props.wide })}>
      {props.children
        ? (
          props.children
        )
        : (
          <>
            <div className={cs.info}>
              <div className={cs.label}>
                {props.label}
                {props.tip && (
                  <Tooltip title={props.tip}>
                    <Icon iconName="HiOutlineInformationCircle" />
                  </Tooltip>
                )}
              </div>
              {props.description && <div className={cs.description}>{props.description}</div>}
            </div>
            <div className={cs.action}>{props.action}</div>
          </>
        )}
    </div>
  );
};
