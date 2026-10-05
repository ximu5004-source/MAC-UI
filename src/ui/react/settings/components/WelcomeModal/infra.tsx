import { Button, Modal } from "antd";
import { z } from "zod";
import { useTranslation } from "react-i18next";
import { persistentSignal } from "libs/ui/react/utils/PersistentSignal";

const schema = z.object({
  welcomeShown: z.boolean().default(false),
  alreadyReviewed: z.boolean().default(false),
  lastReviewPrompt: z.number().nullish(),
});
const state = await persistentSignal("welcomeModal", schema.parse({}), schema);
export function WelcomeModal() {
  const { t } = useTranslation();
  return (
    <Modal
      open={!state.value.welcomeShown}
      centered
      closable={false}
      title={t("mac_ui.welcome_title")}
      footer={
        <Button
          type="primary"
          onClick={() => {
            state.value = { ...state.value, welcomeShown: true };
          }}
        >
          {t("mac_ui.get_started")}
        </Button>
      }
    >
      <p>{t("mac_ui.welcome_body")}</p>
    </Modal>
  );
}
