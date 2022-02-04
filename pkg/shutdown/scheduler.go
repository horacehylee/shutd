package shutdown

import (
	"errors"
	"fmt"
	"time"

	"github.com/go-co-op/gocron"
	"github.com/sirupsen/logrus"
)

const shutdownTag = "shutdown"
const notificationTag = "notification"

type scheduler struct {
	*gocron.Scheduler
	logger *logrus.Logger
}

type option func(*scheduler)

func WithLogger(logger *logrus.Logger) option {
	return func(s *scheduler) {
		s.logger = logger
	}
}

func NewScheduler(options ...option) *scheduler {
	s := gocron.NewScheduler(time.Local)
	s.StartAsync()

	scheduler := &scheduler{
		Scheduler: s,
		logger:    logrus.New(),
	}
	for _, o := range options {
		o(scheduler)
	}
	return scheduler
}

func (s *scheduler) Config(config Config) error {
	s.logger.Infof("config: %#v\n", config)

	s.Clear()
	j, err := s.Every(1).Day().At(config.StartTime).Tag(shutdownTag).Do(func() {
		s.logger.Info("=== Shutdown ===")
		s.printJobs()
		shutdown()
	})
	if err != nil {
		return err
	}

	notifyTime := j.ScheduledTime().Add(-time.Duration(config.Notification.Before) * time.Minute)
	s.Every(1).Day().At(notifyTime).Tag(notificationTag)
	if time.Now().After(notifyTime) {
		s.StartImmediately()
	}
	_, err = s.Do(s.notifyBeforeShutdown(config))
	s.printJobs()
	return err
}

func (s *scheduler) notifyBeforeShutdown(config Config) func() {
	return func() {
		s.logger.Info("=== Notify before shutdown ===")

		j := s.getShutdownJob()
		if j == nil {
			s.logger.Info("could not find shutdown job")
			return
		}

		title := fmt.Sprintf("Shutd - Scheduled Shutdown at %v", j.ScheduledAtTime())
		text := fmt.Sprintf("About to shutdown in %v minutes, wanted to snooze?", config.Notification.Before)
		yes, err := question(title, text, 2*time.Minute)
		if err != nil && !errors.Is(err, questionTimeoutError) {
			s.logger.Errorf("failed to notify before shutdown: %v", err)
			return
		}
		if yes {
			err := s.delay(config.SnoozeInterval)
			if err != nil {
				s.logger.Errorf("failed to delay: %v", err)
			}
		}
		s.printJobs()
	}
}

func (s *scheduler) delay(interval int) error {
	jobs := s.Jobs()
	if len(jobs) == 0 {
		return fmt.Errorf("no scheduled job for scheduler")
	}
	for _, j := range jobs {
		delayedTime := j.ScheduledTime().Add(time.Duration(interval) * time.Minute)
		_, err := s.Job(j).At(delayedTime).Update()
		if err != nil {
			return fmt.Errorf("failed to delay job %w", err)
		}
	}
	s.printJobs()
	return nil
}

func (s *scheduler) getShutdownJob() *gocron.Job {
	for _, j := range s.Jobs() {
		if contains(j.Tags(), shutdownTag) {
			return j
		}
	}
	return nil
}

func contains(s []string, e string) bool {
	for _, a := range s {
		if a == e {
			return true
		}
	}
	return false
}

func (s *scheduler) printJobs() {
	for _, j := range s.Jobs() {
		s.logger.Infof("job: %v, scheduled: %v (%v)\n", j.Tags(), j.ScheduledAtTime(), j.ScheduledTime())
	}
}
