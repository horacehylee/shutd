package shutdown

import (
	"fmt"
	"testing"
	"time"

	"github.com/sirupsen/logrus/hooks/test"
	"github.com/stretchr/testify/assert"
)

func getDefaultConfig() Config {
	return Config{
		StartTime:      "00:00",
		SnoozeInterval: 15,
		Notification: struct {
			Before   int
			Duration int
		}{
			Before:   10,
			Duration: 10,
		},
	}
}

func getConfigWithShutdownTime(t string) Config {
	c := getDefaultConfig()
	c.StartTime = t
	return c
}

func getScheduler(t *testing.T) *Scheduler {
	config := getDefaultConfig()
	s, err := getSchedulerWithConfig(t, config)
	assert.NoError(t, err)
	return s
}

func getSchedulerWithConfig(t *testing.T, config Config, options ...option) (*Scheduler, error) {
	shutdownTask := func(s *Scheduler) error { return nil }
	snoozeNotificationTask := func(s *Scheduler) error { return nil }
	options = append([]option{WithShutdownTask(shutdownTask), WithSnoozeNotificationTask(snoozeNotificationTask)}, options...)
	s, err := NewScheduler(config, options...)
	if err != nil {
		return nil, err
	}

	assert.Equal(t, s.Config(), config)
	return s, nil
}

func TestScheduleJobs(t *testing.T) {
	s := getScheduler(t)
	assert.Equal(t, s.shutdownJob.ScheduledTime().Format("15:04"), "00:00")
	assert.Equal(t, s.shutdownJob.Tags(), []string{"shutdown"})

	assert.Equal(t, s.snoozeNotificationJob.ScheduledTime().Format("15:04"), "23:50")
	assert.Equal(t, s.snoozeNotificationJob.Tags(), []string{"snoozeNotification"})
}

func TestSnooze(t *testing.T) {
	s := getScheduler(t)
	assert.Equal(t, s.shutdownJob.ScheduledTime().Format("15:04"), "00:00")
	assert.Equal(t, s.snoozeNotificationJob.ScheduledTime().Format("15:04"), "23:50")

	err := s.Snooze()
	assert.NoError(t, err)

	assert.Equal(t, s.shutdownJob.ScheduledTime().Format("15:04"), "00:15")
	assert.Equal(t, s.snoozeNotificationJob.ScheduledTime().Format("15:04"), "00:05")
}

func TestConfigureWillUpdateJobTime(t *testing.T) {
	s := getScheduler(t)
	assert.Equal(t, s.shutdownJob.ScheduledTime().Format("15:04"), "00:00")
	assert.Equal(t, s.snoozeNotificationJob.ScheduledTime().Format("15:04"), "23:50")

	err := s.Configure(Config{
		StartTime:      "02:00",
		SnoozeInterval: 15,
		Notification: struct {
			Before   int
			Duration int
		}{
			Before:   10,
			Duration: 10,
		},
	})
	assert.NoError(t, err)

	assert.Equal(t, s.shutdownJob.ScheduledTime().Format("15:04"), "02:00")
	assert.Equal(t, s.snoozeNotificationJob.ScheduledTime().Format("15:04"), "01:50")
}

func TestConfigureWithInvalidTimeFormat(t *testing.T) {
	s := getScheduler(t)
	config := getConfigWithShutdownTime("32:00")
	err := s.Configure(config)
	assert.EqualError(t, err, "failed to update scheduled shutdown job: the given time format is not supported")
}

func TestNewSchedulerWithInvalidTimeFormatConfig(t *testing.T) {
	_, err := getSchedulerWithConfig(t, getConfigWithShutdownTime("32:00"))
	assert.EqualError(t, err, "failed to schedule shutdown job: the given time format is not supported")
}

func TestShutdownTime(t *testing.T) {
	s := getScheduler(t)
	shutdownTime, err := s.ShutdownTime()
	assert.NoError(t, err)
	assert.Equal(t, shutdownTime.Format("15:04"), "00:00")
}

func TestShutdownTimeWithoutShutdownJob(t *testing.T) {
	s := getScheduler(t)
	s.shutdownJob = nil
	_, err := s.ShutdownTime()
	assert.EqualError(t, err, "shutdown job is not scheduled")
}

func TestSchedulingOfSnoozeNotificationWithoutShutdownJob(t *testing.T) {
	s := getScheduler(t)
	s.shutdownJob = nil
	err := s.scheduleSnoozeNotificationJob()
	assert.EqualError(t, err, "shutdown job is not scheduled")
}

func TestSnoozeWithoutShutdownJob(t *testing.T) {
	s := getScheduler(t)
	s.shutdownJob = nil
	err := s.Snooze()
	assert.EqualError(t, err, "shutdown job is not scheduled")
}

func TestSnoozeNotificationStillTriggerBeforeShutdown(t *testing.T) {
	called := make(chan bool)
	snoozeNotificationTask := func(s *Scheduler) error {
		called <- true
		return nil
	}
	config := getConfigWithShutdownTime(time.Now().Add(3 * time.Minute).Format("15:04"))
	_, err := getSchedulerWithConfig(t, config, WithSnoozeNotificationTask(snoozeNotificationTask))
	assert.NoError(t, err)

	select {
	case <-called:
	case <-time.After(2 * time.Second):
		t.Fatal("snoozeNotificationTask should be called")
	}
}

func TestSnoozeNotificationTaskWithError(t *testing.T) {
	testLogger, hook := test.NewNullLogger()

	called := make(chan bool)
	snoozeNotificationTask := func(s *Scheduler) error {
		called <- true
		return fmt.Errorf("testing error")
	}
	config := getConfigWithShutdownTime(time.Now().Add(3 * time.Minute).Format("15:04"))
	_, err := getSchedulerWithConfig(t, config, WithSnoozeNotificationTask(snoozeNotificationTask), WithLogger(testLogger))
	assert.NoError(t, err)

	select {
	case <-called:
		equal := false
		for i := 0; i < 10; i++ {
			equal = assert.Equal(t, hook.LastEntry().Message, "failed to execute snooze notification task: testing error")
			if !equal {
				time.Sleep(100 * time.Millisecond)
			}
		}
		if !equal {
			t.Fatal("error is not thrown for snooze notification task")
		}
	case <-time.After(2 * time.Second):
		t.Fatal("snoozeNotificationTask should be called")
	}
}

func TestShutdownTaskTriggered(t *testing.T) {

	called := make(chan bool)
	shutdownTask := func(s *Scheduler) error {
		called <- true
		return nil
	}
	config := getConfigWithShutdownTime(time.Now().Add(1 * time.Second).Format("15:04:05"))
	_, err := getSchedulerWithConfig(t, config, WithShutdownTask(shutdownTask))
	assert.NoError(t, err)

	select {
	case <-called:
	case <-time.After(2 * time.Second):
		t.Fatal("shutdownTask should be called")
	}
}

func TestShutdownTaskWithError(t *testing.T) {
	testLogger, hook := test.NewNullLogger()

	called := make(chan bool)
	shutdownTask := func(s *Scheduler) error {
		called <- true
		return fmt.Errorf("testing error")
	}
	config := getConfigWithShutdownTime(time.Now().Add(1 * time.Second).Format("15:04:05"))
	_, err := getSchedulerWithConfig(t, config, WithShutdownTask(shutdownTask), WithLogger(testLogger))
	assert.NoError(t, err)

	select {
	case <-called:
		equal := false
		for i := 0; i < 10; i++ {
			equal = assert.Equal(t, hook.LastEntry().Message, "failed to execute shutdown task: testing error")
			if !equal {
				time.Sleep(100 * time.Millisecond)
			}
		}
		if !equal {
			t.Fatal("error is not thrown for shutdown task")
		}
	case <-time.After(2 * time.Second):
		t.Fatal("shutdownTask should be called")
	}
}

func TestShutdownTimeChangedChanShouldGetLatestShutdownTime(t *testing.T) {
	s := getScheduler(t)
	c := s.ShutdownTimeChangedChan()
	select {
	case shutdownTime := <-c:
		assert.Equal(t, shutdownTime.Format("15:04"), "00:00")
	default:
		t.Fatal("shutdownTimeChangedChan should have the latest shutdown time value")
	}

	config := getConfigWithShutdownTime("02:01")
	err := s.Configure(config)
	assert.NoError(t, err)

	select {
	case shutdownTime := <-c:
		assert.Equal(t, shutdownTime.Format("15:04"), "02:01")
	default:
		t.Fatal("shutdownTimeChangedChan should have the latest shutdown time value")
	}
}
