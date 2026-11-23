import { NavLink } from "react-router";
import { useTranslation } from "react-i18next";
import { Icon } from "libs/ui/react/components/Icon";
import { SpotlightCard } from "../../components/react-bits/SpotlightCard";
import { sections } from "../Hub/catalog";
import cs from "./overview.module.css";

export function Home() {
  const { t } = useTranslation();
  return (
    <div className={cs.overview}>
      <div className={cs.intro}>
        <h1>{t("settings_home.title")}</h1>
        <p>{t("settings_home.subtitle")}</p>
      </div>
      <div className={cs.categories}>
        {sections.map((section) => (
          <SpotlightCard key={section.route}>
            <NavLink to={section.route} className={cs.category}>
              <span className={cs.icon} aria-hidden="true">
                <Icon iconName={section.icon} size={25} />
              </span>
              <strong>{t(`settings_home.${section.key}`)}</strong>
              <span className={cs.description}>{t(`settings_home.${section.key}_hint`)}</span>
              <span className={cs.arrow} aria-hidden="true">
                <Icon iconName="TbChevronRight" size={18} />
              </span>
            </NavLink>
          </SpotlightCard>
        ))}
      </div>
      <footer className={cs.footer}>
        <span>MAC UI</span>
        <NavLink to="/extras">
          <Icon iconName="TbInfoCircle" />
          {t("mac_ui.about")}
        </NavLink>
      </footer>
    </div>
  );
}
