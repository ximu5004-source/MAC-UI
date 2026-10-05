<script lang="ts">
  import { globalState } from "./state.svelte";
  import { Widget } from "@seelen-ui/lib";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { t } from "./i18n";
  import moment from "moment";

  const today = moment();

  const momentLang = $derived(globalState.momentLang);

  // svelte-ignore state_referenced_locally
  let date = $state(moment().locale(momentLang));
  // svelte-ignore state_referenced_locally
  let selectedDate = $state(moment().locale(momentLang));
  $effect(() => {
    date = moment().locale(momentLang);
    selectedDate = moment().locale(momentLang);
  });

  const weekDays = $derived.by(() => {
    const weekStart = date.clone().startOf("week");
    return Array.from({ length: 7 }, (_, i) => weekStart.clone().add(i, "days").format("dd"));
  });

  function handlePrevious() {
    const newDate = date.clone().add(-1, globalState.viewMode === "month" ? "months" : "years");
    date = newDate;
  }

  function handleNext() {
    const newDate = date.clone().add(1, globalState.viewMode === "month" ? "months" : "years");
    date = newDate;
  }

  function handleToday() {
    date = moment().locale(momentLang);
    selectedDate = moment().locale(momentLang);
  }

  function toggleViewMode() {
    globalState.viewMode = globalState.viewMode === "month" ? "year" : "month";
  }

  function handleDateSelect(day: moment.Moment) {
    selectedDate = day.clone();
    date = day.clone();
  }

  function handleMonthSelect(month: moment.Moment) {
    date = month;
    globalState.viewMode = "month";
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    e.stopPropagation();

    const isUp = e.deltaY < 0;
    date = date.clone().add(isUp ? 1 : -1, globalState.viewMode === "month" ? "months" : "years");
  }

  // Month view data
  const monthViewData = $derived.by(() => {
    const startOfMonth = date.clone().startOf("month");
    const endOfMonth = date.clone().endOf("month");
    const startDate = startOfMonth.clone().startOf("week");
    const endDate = endOfMonth.clone().endOf("week");

    const weeks: moment.Moment[][] = [];
    let currentWeek: moment.Moment[] = [];
    let currentDate = startDate.clone();

    while (currentDate.isSameOrBefore(endDate, "day")) {
      currentWeek.push(currentDate.clone());
      if (currentWeek.length === 7) {
        weeks.push(currentWeek);
        currentWeek = [];
      }
      currentDate.add(1, "day");
    }

    return weeks;
  });

  // Year view data
  const yearViewData = $derived.by(() => {
    const months: moment.Moment[] = [];

    for (let i = 0; i < 12; i++) {
      months.push(date.clone().month(i).startOf("month"));
    }

    return months;
  });

  $effect(() => {
    Widget.getCurrent().ready();
  });
</script>

<svelte:window onkeydown={(event) => {
  if (event.key === "Escape" && !event.repeat && !event.defaultPrevented) {
    event.preventDefault();
    void Widget.getCurrent().hide();
  }
}} />

<div class="slu-std-popover mac-panel mac-frosted-surface calendar-popup">
  <div class="calendar" onwheel={handleWheel}>
    <!-- Calendar Header -->
    <div class="calendar-header">
      <button
        type="button"
        class="calendar-date"
        onclick={toggleViewMode}
        aria-label={$t(globalState.viewMode === "month" ? "year_view" : "month_view")}
        title={$t(globalState.viewMode === "month" ? "year_view" : "month_view")}
      >
        <span aria-live="polite" aria-atomic="true">
          {globalState.viewMode === "month" ? date.format("MMMM YYYY") : date.format("YYYY")}
        </span>
      </button>
      <div class="calendar-actions">
        <button type="button" class="calendar-navigator" onclick={handlePrevious}
          aria-label={$t(globalState.viewMode === "month" ? "previous_month" : "previous_year")}
          title={$t(globalState.viewMode === "month" ? "previous_month" : "previous_year")}>
          <Icon iconName="AiOutlineLeft" aria-hidden="true" />
        </button>
        <button type="button" class="calendar-navigator" onclick={handleToday}
          aria-label={$t("today")} title={$t("today")}>
          <Icon iconName="AiOutlineHome" aria-hidden="true" />
        </button>
        <button type="button" class="calendar-navigator" onclick={handleNext}
          aria-label={$t(globalState.viewMode === "month" ? "next_month" : "next_year")}
          title={$t(globalState.viewMode === "month" ? "next_month" : "next_year")}>
          <Icon iconName="AiOutlineRight" aria-hidden="true" />
        </button>
      </div>
    </div>

    {#if globalState.viewMode === "month"}
      <!-- Month View -->
      <div class="calendar-month-view">
        <div class="calendar-weekdays">
          {#each weekDays as day}
            <div class="calendar-weekday">{day}</div>
          {/each}
        </div>

        <div class="calendar-days">
          {#each monthViewData as week}
            <div class="calendar-week">
              {#each week as day}
                {@const isToday = day.isSame(today, "day")}
                {@const isSelected = day.isSame(selectedDate, "day")}
                {@const isOffMonth = day.month() !== date.month()}
                <button
                  type="button"
                  class="calendar-cell"
                  class:calendar-cell-today={isToday}
                  class:calendar-cell-selected={isSelected}
                  class:calendar-cell-off-month={isOffMonth}
                  onclick={() => handleDateSelect(day)}
                  aria-label={day.format("LL")}
                  aria-pressed={isSelected}
                  aria-current={isToday ? "date" : undefined}
                >
                  {day.format("D")}
                </button>
              {/each}
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <!-- Year View -->
      <div class="calendar-year-view">
        {#each yearViewData as month}
          {@const isCurrentMonth = month.isSame(today, "month")}
          <button
            type="button"
            class="calendar-month-cell"
            class:calendar-month-cell-current={isCurrentMonth}
            onclick={() => handleMonthSelect(month)}
            aria-label={month.format("MMMM YYYY")}
          >
            {month.format("MMMM")}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>
